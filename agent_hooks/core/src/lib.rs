//! Core check functions for `agent_hooks`.
//!
//! This library provides simple, reusable check functions that can be used by
//! any AI coding agent (Claude Code, `OpenCode`, etc.) to implement safety hooks.

use regex::Regex;
use std::sync::LazyLock;

// ============================================================================
// rm command detection
// ============================================================================

#[cfg(not(windows))]
static RM_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(^|[;&|()]\s*)(sudo\s+)?(command\s+)?(\\)?(\S*/)?rm(\s|$)").unwrap()
});

#[cfg(windows)]
static RM_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)(^|[;&|()]\s*)(sudo\s+)?(command\s+)?(\\)?(\S*[\\/])?(rm|del|rd|rmdir|remove-item)(\s|$)",
    )
    .unwrap()
});

/// Check if a command contains an rm (or equivalent) command.
///
/// Returns `true` if the command should be blocked.
#[must_use]
pub fn is_rm_command(cmd: &str) -> bool {
    RM_PATTERN.is_match(cmd)
}

// ============================================================================
// Destructive find command detection
// ============================================================================

#[cfg(not(windows))]
const DESTRUCTIVE_PATTERNS: &[(&str, &str); 6] = &[
    (r"find\s+.*-delete", "find with -delete option"),
    (
        r"find\s+.*-exec\s+(sudo\s+)?(rm|rmdir)\s",
        "find with -exec rm/rmdir",
    ),
    (
        r"find\s+.*-execdir\s+(sudo\s+)?(rm|rmdir)\s",
        "find with -execdir rm/rmdir",
    ),
    (
        r"find\s+.*\|\s*(sudo\s+)?xargs\s+(sudo\s+)?(rm|rmdir)",
        "find piped to xargs rm/rmdir",
    ),
    (r"find\s+.*-exec\s+(sudo\s+)?mv\s", "find with -exec mv"),
    (
        r"find\s+.*-ok\s+(sudo\s+)?(rm|rmdir)\s",
        "find with -ok rm/rmdir",
    ),
];

#[cfg(windows)]
const DESTRUCTIVE_PATTERNS: &[(&str, &str); 1] =
    &[(r"\|\s*(move|move-item)\b", "piped to move/move-item")];

#[cfg(not(windows))]
static FIND_CHECK: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(^|[;&|()]\s*)find\s").unwrap());

#[cfg(windows)]
static FIND_CHECK: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\|").unwrap());

/// Check if a command is a destructive find command.
///
/// Returns `Some(description)` if the command is destructive and should be confirmed,
/// or `None` if the command is safe.
#[must_use]
#[expect(clippy::missing_panics_doc)]
pub fn check_destructive_find(cmd: &str) -> Option<&'static str> {
    if !FIND_CHECK.is_match(cmd) {
        return None;
    }

    for (pattern, description) in DESTRUCTIVE_PATTERNS {
        let re = Regex::new(&format!("(?i){pattern}")).unwrap();
        if re.is_match(cmd) {
            return Some(description);
        }
    }

    None
}

// ============================================================================
// Rust #[allow(...)] / #[expect(...)] detection
// ============================================================================

static RUST_ALLOW_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"#!?\[allow\s*\(").unwrap());

static RUST_EXPECT_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"#!?\[expect\s*\(").unwrap());

/// Check if a position in the content is inside a line comment or string literal.
fn is_in_comment_or_string(content: &str, match_start: usize) -> bool {
    let before = &content[..match_start];

    // Check if in line comment (// ...)
    let line_start = before.rfind('\n').map_or(0, |p| p + 1);
    let current_line = &before[line_start..];
    if current_line.contains("//") {
        return true;
    }

    // Check if inside a block comment (/* ... */)
    let block_open = before.matches("/*").count();
    let block_close = before.matches("*/").count();
    if block_open > block_close {
        return true;
    }

    // Check if inside a string literal
    let mut in_raw_string = false;
    let mut i = 0;
    let bytes = before.as_bytes();
    while i < bytes.len() {
        if in_raw_string {
            if bytes[i] == b'"' {
                in_raw_string = false;
            }
        } else {
            if bytes[i] == b'r' && i + 1 < bytes.len() {
                let mut j = i + 1;
                while j < bytes.len() && bytes[j] == b'#' {
                    j += 1;
                }
                if j < bytes.len() && bytes[j] == b'"' {
                    in_raw_string = true;
                    i = j + 1;
                    continue;
                }
            }
            if bytes[i] == b'"' && (i == 0 || bytes[i - 1] != b'\\') {
                let mut k = i + 1;
                while k < bytes.len() {
                    if bytes[k] == b'"' && bytes[k - 1] != b'\\' {
                        break;
                    }
                    k += 1;
                }
                if k >= bytes.len() {
                    return true;
                }
                i = k + 1;
                continue;
            }
        }
        i += 1;
    }

    in_raw_string
}

/// Find if there are real matches of a pattern (not in comments or strings).
#[inline]
fn find_real_matches(content: &str, pattern: &Regex) -> bool {
    for m in pattern.find_iter(content) {
        if !is_in_comment_or_string(content, m.start()) {
            return true;
        }
    }
    false
}

/// Result of checking for Rust allow/expect attributes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RustAllowCheckResult {
    /// No problematic attributes found.
    Ok,
    /// Found #[allow(...)] attribute.
    HasAllow,
    /// Found #[expect(...)] attribute.
    HasExpect,
    /// Found both #[allow(...)] and #[expect(...)] attributes.
    HasBoth,
}

/// Check if content contains #[allow(...)] or #[expect(...)] attributes.
///
/// This function ignores attributes in comments and string literals.
/// It does NOT check if the file is a Rust file - the caller should do that.
#[must_use]
pub fn check_rust_allow_attributes(content: &str) -> RustAllowCheckResult {
    let has_allow = find_real_matches(content, &RUST_ALLOW_PATTERN);
    let has_expect = find_real_matches(content, &RUST_EXPECT_PATTERN);

    match (has_allow, has_expect) {
        (true, true) => RustAllowCheckResult::HasBoth,
        (true, false) => RustAllowCheckResult::HasAllow,
        (false, true) => RustAllowCheckResult::HasExpect,
        (false, false) => RustAllowCheckResult::Ok,
    }
}

/// Check if a file path is a Rust file.
#[must_use]
pub fn is_rust_file(file_path: &str) -> bool {
    std::path::Path::new(file_path)
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("rs"))
}

#[cfg(test)]
mod tests;
