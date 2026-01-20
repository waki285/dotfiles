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
    // Match: rm command (direct) or xargs rm/rmdir (piped)
    Regex::new(
        r"(^|[;&|()]\s*)(sudo\s+)?(command\s+)?(\\)?(\S*/)?(rm|xargs\s+(sudo\s+)?(rm|rmdir))(\s|$)",
    )
    .unwrap()
});

#[cfg(windows)]
static RM_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    // Match: rm/del/rd/rmdir/remove-item command (direct) or xargs rm/rmdir (piped)
    Regex::new(
        r"(?i)(^|[;&|()]\s*)(sudo\s+)?(command\s+)?(\\)?(\S*[\\/])?(rm|del|rd|rmdir|remove-item|xargs\s+(sudo\s+)?(rm|rmdir))(\s|$)",
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

// ============================================================================
// Dangerous path detection for rm/trash/mv commands
// ============================================================================

/// Result of checking for dangerous path operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DangerousPathCheck {
    /// The dangerous path that was matched.
    pub matched_path: String,
    /// The command type (rm, trash, mv).
    pub command_type: String,
}

/// Expand ~ to home directory in a path.
fn expand_home(path: &str) -> String {
    if path.starts_with("~/")
        && let Some(home) = std::env::var_os("HOME")
    {
        return format!("{}{}", home.to_string_lossy(), &path[1..]);
    }
    path.to_string()
}

/// Normalize a path for comparison (expand ~, resolve . and .., but don't require existence).
fn normalize_path(path: &str) -> String {
    let expanded = expand_home(path);
    // Use canonicalize if the path exists, otherwise just use the expanded path
    std::fs::canonicalize(&expanded)
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or(expanded)
}

/// Check if a path matches a dangerous path pattern.
///
/// - If dangerous path ends with `/` (e.g., `~/`), only match exact directory or wildcards
/// - Otherwise, match the path exactly or as a prefix
fn is_dangerous_path(path: &str, dangerous_paths: &[&str]) -> Option<String> {
    // Check for wildcard patterns first (these are always dangerous)
    let has_wildcard = path.contains('*') || path.contains('?');

    for &dangerous in dangerous_paths {
        if dangerous.ends_with('/') {
            // Directory pattern (e.g., "~/")
            // Only match:
            // 1. Exact directory (e.g., "~/" or "~/.")
            // 2. Wildcard patterns (e.g., "~/*", "~/.*")
            let dangerous_base = dangerous.trim_end_matches('/');
            let path_trimmed = path.trim_end_matches('/');

            // Exact match (e.g., "~" or "~/")
            if path_trimmed == dangerous_base || path == dangerous {
                return Some(dangerous.to_string());
            }

            // Wildcard in the dangerous directory (e.g., "~/*", "~/.*")
            if has_wildcard {
                let expanded_dangerous = expand_home(dangerous);
                let expanded_path = expand_home(path);

                // Check if wildcard is directly under the dangerous directory
                // e.g., "~/*" matches, but "~/Documents/*" does not
                if let Some(rest) =
                    expanded_path.strip_prefix(expanded_dangerous.trim_end_matches('/'))
                {
                    // rest should be like "/*" or "/.*" (wildcard directly under)
                    if let Some(after_slash) = rest.strip_prefix('/') {
                        // Only match if it's a direct wildcard (no subdirectory)
                        if !after_slash.contains('/')
                            && (after_slash.contains('*') || after_slash.contains('?'))
                        {
                            return Some(dangerous.to_string());
                        }
                    }
                }
            }
        } else {
            // Exact path pattern (e.g., "/etc/passwd")
            let normalized = normalize_path(path);
            let dangerous_normalized = normalize_path(dangerous);

            if normalized == dangerous_normalized
                || normalized.starts_with(&format!("{dangerous_normalized}/"))
            {
                return Some(dangerous.to_string());
            }
        }
    }

    None
}

/// Check if a bash command targets dangerous paths with rm/trash/mv.
///
/// Returns `Some(DangerousPathCheck)` if a dangerous operation is detected.
#[must_use]
pub fn check_dangerous_path_command(
    cmd: &str,
    dangerous_paths: &[&str],
) -> Option<DangerousPathCheck> {
    // Patterns to match rm, trash, mv commands and extract their arguments
    // We look for these commands and then check their path arguments

    let cmd_trimmed = cmd.trim();

    // Split by common command separators to handle chained commands
    let segments: Vec<&str> = cmd_trimmed.split([';', '&', '|']).collect();

    for segment in segments {
        let segment = segment.trim();
        if segment.is_empty() {
            continue;
        }

        // Remove leading sudo if present
        let segment = segment.strip_prefix("sudo ").unwrap_or(segment).trim();

        // Check for rm, trash, or mv commands
        let (cmd_type, args) = if let Some(rest) = segment.strip_prefix("rm ") {
            ("rm", rest)
        } else if let Some(rest) = segment.strip_prefix("trash ") {
            ("trash", rest)
        } else if let Some(rest) = segment.strip_prefix("mv ") {
            ("mv", rest)
        } else {
            continue;
        };

        // Parse arguments, skipping flags (starting with -)
        for arg in args.split_whitespace() {
            if arg.starts_with('-') {
                continue;
            }

            // Check if this path is dangerous
            if let Some(matched) = is_dangerous_path(arg, dangerous_paths) {
                return Some(DangerousPathCheck {
                    matched_path: matched,
                    command_type: cmd_type.to_string(),
                });
            }
        }
    }

    None
}

// ============================================================================
// Package manager mismatch detection
// ============================================================================

/// Represents a JavaScript/Node.js package manager.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackageManager {
    Npm,
    Pnpm,
    Yarn,
    Bun,
}

impl PackageManager {
    /// Returns the display name of the package manager.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Npm => "npm",
            Self::Pnpm => "pnpm",
            Self::Yarn => "yarn",
            Self::Bun => "bun",
        }
    }

    /// Returns the lock file name(s) for this package manager.
    #[must_use]
    pub const fn lock_files(self) -> &'static [&'static str] {
        match self {
            Self::Npm => &["package-lock.json"],
            Self::Pnpm => &["pnpm-lock.yaml"],
            Self::Yarn => &["yarn.lock"],
            Self::Bun => &["bun.lockb", "bun.lock"],
        }
    }
}

const ALL_PACKAGE_MANAGERS: &[PackageManager] = &[
    PackageManager::Npm,
    PackageManager::Pnpm,
    PackageManager::Yarn,
    PackageManager::Bun,
];

/// Result of checking for package manager mismatch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackageManagerCheckResult {
    /// No package manager command detected or no lock file found.
    Ok,
    /// Command matches the lock file's package manager.
    Matching,
    /// Command uses a different package manager than the lock file indicates.
    /// Should deny this operation.
    Mismatch {
        /// The package manager being used in the command.
        command_pm: PackageManager,
        /// The package manager indicated by the lock file.
        expected_pm: PackageManager,
    },
    /// Multiple lock files exist, so we can't determine the correct package manager.
    /// Should ask the user instead of denying.
    Ambiguous {
        /// The package manager being used in the command.
        command_pm: PackageManager,
        /// The package managers that have lock files present.
        detected_pms: Vec<PackageManager>,
    },
}

/// Regex patterns for detecting package manager commands.
static PM_COMMAND_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    // Match npm/pnpm/yarn/bun followed by install/add/remove/ci/update/upgrade/uninstall/link/rebuild/dedupe
    Regex::new(
        r"(?:^|[;&|()]\s*)(?:sudo\s+)?(?:npx\s+)?(?P<pm>npm|pnpm|yarn|bun)\s+(?P<subcmd>install|add|remove|uninstall|ci|update|upgrade|link|rebuild|dedupe|i|rm|un|up)(?:\s|$)",
    )
    .unwrap()
});

/// Detect which package manager a command is trying to use.
#[must_use]
pub fn detect_package_manager_command(cmd: &str) -> Option<PackageManager> {
    PM_COMMAND_PATTERN.captures(cmd).and_then(|caps| {
        caps.name("pm").map(|m| match m.as_str() {
            "npm" => PackageManager::Npm,
            "pnpm" => PackageManager::Pnpm,
            "yarn" => PackageManager::Yarn,
            "bun" => PackageManager::Bun,
            _ => unreachable!(),
        })
    })
}

/// Find lock files starting from `start_dir` and searching up to parent directories.
///
/// Returns a list of package managers whose lock files were found.
#[must_use]
pub fn find_lock_files(start_dir: &std::path::Path) -> Vec<PackageManager> {
    let mut current = Some(start_dir);
    while let Some(dir) = current {
        let mut found = Vec::new();
        for &pm in ALL_PACKAGE_MANAGERS {
            for &lock_file in pm.lock_files() {
                if dir.join(lock_file).exists() {
                    found.push(pm);
                    break;
                }
            }
        }
        if !found.is_empty() {
            return found;
        }
        current = dir.parent();
    }
    Vec::new()
}

/// Check if a bash command uses a mismatched package manager.
///
/// # Arguments
/// * `cmd` - The bash command to check.
/// * `start_dir` - The directory to start searching for lock files.
///
/// # Returns
/// * `PackageManagerCheckResult::Ok` - No package manager command detected or no lock file found.
/// * `PackageManagerCheckResult::Matching` - Command matches the detected package manager.
/// * `PackageManagerCheckResult::Mismatch` - Command uses wrong package manager (should deny).
/// * `PackageManagerCheckResult::Ambiguous` - Multiple lock files exist (should ask).
#[must_use]
pub fn check_package_manager(cmd: &str, start_dir: &std::path::Path) -> PackageManagerCheckResult {
    let Some(command_pm) = detect_package_manager_command(cmd) else {
        return PackageManagerCheckResult::Ok;
    };

    let detected_pms = find_lock_files(start_dir);

    if detected_pms.is_empty() {
        return PackageManagerCheckResult::Ok;
    }

    if detected_pms.len() > 1 {
        return PackageManagerCheckResult::Ambiguous {
            command_pm,
            detected_pms,
        };
    }

    let expected_pm = detected_pms[0];
    if command_pm == expected_pm {
        PackageManagerCheckResult::Matching
    } else {
        PackageManagerCheckResult::Mismatch {
            command_pm,
            expected_pm,
        }
    }
}

#[cfg(test)]
mod tests;
