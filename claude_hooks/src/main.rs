use regex::Regex;
use seahorse::{App, Command, Context, Flag, FlagType};
use serde::{Deserialize, Serialize};
use std::{
    io::{self, Read},
    sync::LazyLock,
};

// ============================================================================
// Enums for type safety
// ============================================================================

/// Hook event names that can be returned in the output
#[derive(Debug, Clone, Copy, Serialize)]
pub enum HookEventName {
    PermissionRequest,
    PreToolUse,
}

/// Behavior for permission decisions
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DecisionBehavior {
    Deny,
    Allow,
}

/// Permission decision types for ask behavior
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PermissionDecision {
    Ask,
    Allow,
    Deny,
}

// ============================================================================
// Input structures
// ============================================================================

/// Input received from Claude Code hooks via stdin
#[derive(Debug, Deserialize)]
pub struct HookInput {
    pub tool_name: Option<String>,
    pub tool_input: Option<ToolInput>,
}

#[derive(Debug, Deserialize)]
pub struct ToolInput {
    pub command: Option<String>,
    /// For Edit tool: the new content to replace
    pub new_string: Option<String>,
    /// For Write tool: the content to write
    pub content: Option<String>,
    /// For Edit/Write tools: the file path
    pub file_path: Option<String>,
}

// ============================================================================
// Output structures
// ============================================================================

/// Output to be printed as JSON to stdout
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HookOutput {
    pub hook_specific_output: HookSpecificOutput,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HookSpecificOutput {
    pub hook_event_name: HookEventName,

    /// Used for deny behavior (block-rm)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decision: Option<Decision>,

    /// Used for ask behavior (confirm-destructive-find)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permission_decision: Option<PermissionDecision>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub permission_decision_reason: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct Decision {
    pub behavior: DecisionBehavior,
    pub message: String,
}

// ============================================================================
// Helper functions
// ============================================================================

fn read_hook_input() -> io::Result<HookInput> {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input)?;
    serde_json::from_str(&input).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

fn output_hook_result(output: &HookOutput) {
    if let Ok(json) = serde_json::to_string(output) {
        println!("{json}");
    }
}

// ============================================================================
// Hook implementations
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

/// Block rm command and suggest using trash instead
fn block_rm(cmd: &str) -> Option<HookOutput> {
    if RM_PATTERN.is_match(cmd) {
        return Some(HookOutput {
            hook_specific_output: HookSpecificOutput {
                hook_event_name: HookEventName::PermissionRequest,
                decision: Some(Decision {
                    behavior: DecisionBehavior::Deny,
                    message: "rm is forbidden. Use trash command to delete files. Example: trash <path...>".to_string(),
                }),
                permission_decision: None,
                permission_decision_reason: None,
            },
        });
    }

    None
}

// Destructive patterns with descriptions
#[cfg(not(windows))]
const DESTRUCTIVE_PATTERNS: &[(&str, &str); 6] = &[
    // find ... -delete
    (r"find\s+.*-delete", "find with -delete option"),
    // find ... -exec rm/rmdir ...
    (
        r"find\s+.*-exec\s+(sudo\s+)?(rm|rmdir)\s",
        "find with -exec rm/rmdir",
    ),
    // find ... -execdir rm/rmdir ...
    (
        r"find\s+.*-execdir\s+(sudo\s+)?(rm|rmdir)\s",
        "find with -execdir rm/rmdir",
    ),
    // find ... | xargs rm/rmdir
    (
        r"find\s+.*\|\s*(sudo\s+)?xargs\s+(sudo\s+)?(rm|rmdir)",
        "find piped to xargs rm/rmdir",
    ),
    // find ... -exec mv ...
    (r"find\s+.*-exec\s+(sudo\s+)?mv\s", "find with -exec mv"),
    // find ... -ok rm/rmdir ...
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

/// Confirm destructive find commands
fn confirm_destructive_find(cmd: &str) -> Option<HookOutput> {
    // First check if this is a find command
    if !FIND_CHECK.is_match(cmd) {
        return None;
    }

    for (pattern, description) in DESTRUCTIVE_PATTERNS {
        let re = Regex::new(&format!("(?i){pattern}")).unwrap();
        if re.is_match(cmd) {
            return Some(HookOutput {
                hook_specific_output: HookSpecificOutput {
                    hook_event_name: HookEventName::PermissionRequest,
                    decision: None,
                    permission_decision: Some(PermissionDecision::Ask),
                    permission_decision_reason: Some(format!(
                        "Destructive find command detected: {description}. \
                         This operation may delete or modify files. Please confirm."
                    )),
                },
            });
        }
    }

    None
}

// ============================================================================
// Rust #[allow(...)] / #[expect(...)] detection for PreToolUse (Edit/Write)
// ============================================================================

/// Pattern to detect #[allow(...)] or #![allow(...)] attributes in Rust code
static RUST_ALLOW_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"#!?\[allow\s*\(").unwrap());

/// Pattern to detect #[expect(...)] or #![expect(...)] attributes in Rust code
static RUST_EXPECT_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"#!?\[expect\s*\(").unwrap());

/// Check if a position in the content is inside a line comment or string literal
fn is_in_comment_or_string(content: &str, match_start: usize) -> bool {
    let before = &content[..match_start];

    // Check if in line comment (// ...)
    // Find the last newline before the match
    let line_start = before.rfind('\n').map_or(0, |p| p + 1);
    let current_line = &before[line_start..];
    if current_line.contains("//") {
        return true;
    }

    // Check if inside a block comment (/* ... */)
    // Count /* and */ before the position
    let block_open = before.matches("/*").count();
    let block_close = before.matches("*/").count();
    if block_open > block_close {
        return true;
    }

    // Check if inside a string literal
    // This is a simplified check - count unescaped quotes
    // For raw strings r#"..."#, we do a simple heuristic

    // Check for raw string r#"..."# - look for unclosed r#" or r"
    // Find the last r#" or r" that isn't closed
    let mut in_raw_string = false;
    let mut i = 0;
    let bytes = before.as_bytes();
    while i < bytes.len() {
        if in_raw_string {
            // Inside raw string - look for closing "# pattern
            if bytes[i] == b'"' {
                // This could be the end - raw strings end with "# (matching # count)
                // Simplified: just assume any " might end it
                in_raw_string = false;
            }
        } else {
            // Check for raw string start: r" or r#" or r##" etc.
            if bytes[i] == b'r' && i + 1 < bytes.len() {
                let mut j = i + 1;
                // Count # signs
                while j < bytes.len() && bytes[j] == b'#' {
                    j += 1;
                }
                if j < bytes.len() && bytes[j] == b'"' {
                    in_raw_string = true;
                    i = j + 1;
                    continue;
                }
            }
            // Check for regular string
            if bytes[i] == b'"' && (i == 0 || bytes[i - 1] != b'\\') {
                // Toggle string state - but we need to find the closing quote
                let mut k = i + 1;
                while k < bytes.len() {
                    if bytes[k] == b'"' && bytes[k - 1] != b'\\' {
                        break;
                    }
                    k += 1;
                }
                if k >= bytes.len() {
                    // Unclosed string
                    return true;
                }
                i = k + 1;
                continue;
            }
        }
        i += 1;
    }

    if in_raw_string {
        return true;
    }

    false
}

/// Find all matches of a pattern that are not in comments or strings
fn find_real_matches(content: &str, pattern: &Regex) -> bool {
    for m in pattern.find_iter(content) {
        if !is_in_comment_or_string(content, m.start()) {
            return true;
        }
    }
    false
}

/// Options for `deny_rust_allow` hook
pub struct DenyRustAllowOptions {
    /// If true, suggest using #[expect(...)] instead of #[allow(...)]
    /// If false, deny both #[allow(...)] and #[expect(...)]
    pub expect: bool,
    /// Additional context message to append to the denial reason
    pub additional_context: Option<String>,
}

/// Deny adding #[allow(...)] or #![allow(...)] attributes to Rust files
/// Returns `PreToolUse` format output
fn deny_rust_allow(
    tool_name: &str,
    tool_input: &ToolInput,
    options: &DenyRustAllowOptions,
) -> Option<HookOutput> {
    // Only check Edit and Write tools
    if tool_name != "Edit" && tool_name != "Write" {
        return None;
    }

    // Check if this is a Rust file
    let file_path = tool_input.file_path.as_deref().unwrap_or_default();
    if !std::path::Path::new(file_path)
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("rs"))
    {
        return None;
    }

    // Get the content being written/edited
    let content = tool_input
        .new_string
        .as_deref()
        .or(tool_input.content.as_deref())
        .unwrap_or_default();

    if content.is_empty() {
        return None;
    }

    // Use find_real_matches to ignore comments and string literals
    let has_allow = find_real_matches(content, &RUST_ALLOW_PATTERN);
    let has_expect = find_real_matches(content, &RUST_EXPECT_PATTERN);

    // Build the denial message based on options
    let denial_reason = if options.expect {
        // --expect=true: only deny #[allow], suggest using #[expect] instead
        if has_allow {
            let mut msg = "Adding #[allow(...)] or #![allow(...)] attributes is not permitted. \
                           Use #[expect(...)] instead, which will warn when the lint is no longer triggered."
                .to_string();
            if let Some(ref ctx) = options.additional_context {
                msg.push(' ');
                msg.push_str(ctx);
            }
            Some(msg)
        } else {
            None
        }
    } else {
        // no --expect: deny both #[allow] and #[expect]
        if has_allow || has_expect {
            let mut msg = if has_allow && has_expect {
                "Adding #[allow(...)] or #[expect(...)] attributes is not permitted. \
                 Fix the underlying issue instead of suppressing the warning."
                    .to_string()
            } else if has_allow {
                "Adding #[allow(...)] or #![allow(...)] attributes is not permitted. \
                 Fix the underlying issue instead of suppressing the warning."
                    .to_string()
            } else {
                "Adding #[expect(...)] or #![expect(...)] attributes is not permitted. \
                 Fix the underlying issue instead of suppressing the warning."
                    .to_string()
            };
            if let Some(ref ctx) = options.additional_context {
                msg.push(' ');
                msg.push_str(ctx);
            }
            Some(msg)
        } else {
            None
        }
    };

    denial_reason.map(|reason| HookOutput {
        hook_specific_output: HookSpecificOutput {
            hook_event_name: HookEventName::PreToolUse,
            decision: None,
            permission_decision: Some(PermissionDecision::Deny),
            permission_decision_reason: Some(reason),
        },
    })
}

// ============================================================================
// Command handlers
// ============================================================================

fn permission_request_action(_c: &Context) {
    let Ok(data) = read_hook_input() else {
        return;
    };

    let tool_name = data.tool_name.as_deref().unwrap_or_default();
    if tool_name != "Bash" {
        return;
    }

    let cmd = data
        .tool_input
        .as_ref()
        .and_then(|ti| ti.command.as_deref())
        .unwrap_or_default();

    if cmd.is_empty() {
        return;
    }

    if let Some(output) = block_rm(cmd).or_else(|| confirm_destructive_find(cmd)) {
        output_hook_result(&output);
    }
}

fn deny_rust_allow_action(c: &Context) {
    let Ok(data) = read_hook_input() else {
        return;
    };

    let tool_name = data.tool_name.as_deref().unwrap_or_default();
    if tool_name != "Edit" && tool_name != "Write" {
        return;
    }

    let Some(tool_input) = data.tool_input.as_ref() else {
        return;
    };

    // Parse flags
    let expect = c.bool_flag("expect");
    let additional_context = c.string_flag("additional-context").ok();

    let options = DenyRustAllowOptions {
        expect,
        additional_context,
    };

    if let Some(output) = deny_rust_allow(tool_name, tool_input, &options) {
        output_hook_result(&output);
    }
}

// ============================================================================
// Main
// ============================================================================

fn main() {
    let args: Vec<String> = std::env::args().collect();

    let app = App::new(env!("CARGO_PKG_NAME"))
        .description(env!("CARGO_PKG_DESCRIPTION"))
        .version(env!("CARGO_PKG_VERSION"))
        .command(
            Command::new("permission-request")
                .description("Check and handle permission requests for Bash commands")
                .action(permission_request_action),
        )
        .command(
            Command::new("deny-rust-allow")
                .description("Deny #[allow(...)] attributes in Rust files (Edit/Write)")
                .flag(
                    Flag::new("expect", FlagType::Bool)
                        .description("If true, suggest #[expect(...)] instead of denying. If false (default), deny both #[allow] and #[expect]"),
                )
                .flag(
                    Flag::new("additional-context", FlagType::String)
                        .description("Additional context message to append to the denial reason"),
                )
                .action(deny_rust_allow_action),
        );

    app.run(args);
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // -------------------------------------------------------------------------
    // Helper functions tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_is_in_comment_or_string_line_comment() {
        let content = "// #[allow(dead_code)]";
        assert!(is_in_comment_or_string(content, 3));
    }

    #[test]
    fn test_is_in_comment_or_string_not_in_comment() {
        let content = "#[allow(dead_code)]";
        assert!(!is_in_comment_or_string(content, 0));
    }

    #[test]
    fn test_is_in_comment_or_string_block_comment() {
        let content = "/* #[allow(dead_code)] */";
        assert!(is_in_comment_or_string(content, 3));
    }

    #[test]
    fn test_is_in_comment_or_string_string_literal() {
        // Content: let s = "#[allow(dead_code)]";
        let content = "let s = \"#[allow(dead_code)]\";";
        assert!(is_in_comment_or_string(content, 9));
    }

    #[test]
    fn test_is_in_comment_or_string_after_comment() {
        let content = "// comment\n#[allow(dead_code)]";
        assert!(!is_in_comment_or_string(content, 11));
    }

    #[test]
    fn test_find_real_matches_ignores_comments() {
        let content = "// #[allow(dead_code)]\nfn foo() {}";
        assert!(!find_real_matches(content, &RUST_ALLOW_PATTERN));
    }

    #[test]
    fn test_find_real_matches_detects_real_allow() {
        let content = "#[allow(dead_code)]\nfn foo() {}";
        assert!(find_real_matches(content, &RUST_ALLOW_PATTERN));
    }

    #[test]
    fn test_find_real_matches_after_comment() {
        let content = "// comment\n#[allow(dead_code)]";
        assert!(find_real_matches(content, &RUST_ALLOW_PATTERN));
    }

    // -------------------------------------------------------------------------
    // block_rm tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_block_rm_simple() {
        assert!(block_rm("rm file.txt").is_some());
    }

    #[test]
    fn test_block_rm_with_flags() {
        assert!(block_rm("rm -rf /tmp/test").is_some());
    }

    #[test]
    fn test_block_rm_with_sudo() {
        assert!(block_rm("sudo rm -rf /").is_some());
    }

    #[test]
    fn test_block_rm_in_pipeline() {
        assert!(block_rm("echo test && rm file.txt").is_some());
    }

    #[test]
    fn test_block_rm_allows_other_commands() {
        assert!(block_rm("ls -la").is_none());
        assert!(block_rm("trash file.txt").is_none());
    }

    #[test]
    fn test_block_rm_allows_grep_rm() {
        // "rm" as part of another word should not match
        assert!(block_rm("grep -r 'pattern' .").is_none());
        assert!(block_rm("rma -rm").is_none());
    }

    // -------------------------------------------------------------------------
    // confirm_destructive_find tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_confirm_destructive_find_delete() {
        let result = confirm_destructive_find("find . -name '*.tmp' -delete");
        assert!(result.is_some());
        let output = result.unwrap();
        assert!(matches!(
            output.hook_specific_output.permission_decision,
            Some(PermissionDecision::Ask)
        ));
    }

    #[test]
    fn test_confirm_destructive_find_exec_rm() {
        let result = confirm_destructive_find("find . -exec rm {} \\;");
        assert!(result.is_some());
    }

    #[test]
    fn test_confirm_destructive_find_xargs_rm() {
        let result = confirm_destructive_find("find . -name '*.tmp' | xargs rm");
        assert!(result.is_some());
    }

    #[test]
    fn test_confirm_destructive_find_safe() {
        assert!(confirm_destructive_find("find . -name '*.rs'").is_none());
        assert!(confirm_destructive_find("find . -type f -print").is_none());
    }

    // -------------------------------------------------------------------------
    // deny_rust_allow tests
    // -------------------------------------------------------------------------

    fn make_tool_input(file_path: &str, new_string: &str) -> ToolInput {
        ToolInput {
            command: None,
            new_string: Some(new_string.to_string()),
            content: None,
            file_path: Some(file_path.to_string()),
        }
    }

    fn default_options() -> DenyRustAllowOptions {
        DenyRustAllowOptions {
            expect: false,
            additional_context: None,
        }
    }

    #[test]
    fn test_deny_rust_allow_detects_allow() {
        let input = make_tool_input("src/main.rs", "#[allow(dead_code)]\nfn foo() {}");
        let result = deny_rust_allow("Edit", &input, &default_options());
        assert!(result.is_some());
        let output = result.unwrap();
        assert!(matches!(
            output.hook_specific_output.permission_decision,
            Some(PermissionDecision::Deny)
        ));
    }

    #[test]
    fn test_deny_rust_allow_detects_inner_allow() {
        let input = make_tool_input("src/lib.rs", "#![allow(unused)]");
        let result = deny_rust_allow("Write", &input, &default_options());
        assert!(result.is_some());
    }

    #[test]
    fn test_deny_rust_allow_detects_expect_without_flag() {
        let input = make_tool_input("src/main.rs", "#[expect(dead_code)]");
        let result = deny_rust_allow("Edit", &input, &default_options());
        assert!(result.is_some()); // Should deny #[expect] too
    }

    #[test]
    fn test_deny_rust_allow_allows_expect_with_flag() {
        let input = make_tool_input("src/main.rs", "#[expect(dead_code)]");
        let options = DenyRustAllowOptions {
            expect: true,
            additional_context: None,
        };
        let result = deny_rust_allow("Edit", &input, &options);
        assert!(result.is_none()); // Should allow #[expect]
    }

    #[test]
    fn test_deny_rust_allow_denies_allow_with_expect_flag() {
        let input = make_tool_input("src/main.rs", "#[allow(dead_code)]");
        let options = DenyRustAllowOptions {
            expect: true,
            additional_context: None,
        };
        let result = deny_rust_allow("Edit", &input, &options);
        assert!(result.is_some()); // Should still deny #[allow]
    }

    #[test]
    fn test_deny_rust_allow_ignores_non_rust_files() {
        let input = make_tool_input("README.md", "#[allow(dead_code)]");
        let result = deny_rust_allow("Edit", &input, &default_options());
        assert!(result.is_none());
    }

    #[test]
    fn test_deny_rust_allow_ignores_comments() {
        let input = make_tool_input("src/main.rs", "// #[allow(dead_code)]\nfn foo() {}");
        let result = deny_rust_allow("Edit", &input, &default_options());
        assert!(result.is_none());
    }

    #[test]
    fn test_deny_rust_allow_ignores_string_literals() {
        // Content: let s = "#[allow(dead_code)]";
        let input = make_tool_input("src/main.rs", "let s = \"#[allow(dead_code)]\";");
        let result = deny_rust_allow("Edit", &input, &default_options());
        assert!(result.is_none());
    }

    #[test]
    fn test_deny_rust_allow_ignores_wrong_tool() {
        let input = make_tool_input("src/main.rs", "#[allow(dead_code)]");
        let result = deny_rust_allow("Bash", &input, &default_options());
        assert!(result.is_none());
    }

    #[test]
    fn test_deny_rust_allow_additional_context() {
        let input = make_tool_input("src/main.rs", "#[allow(dead_code)]");
        let options = DenyRustAllowOptions {
            expect: false,
            additional_context: Some("See guidelines".to_string()),
        };
        let result = deny_rust_allow("Edit", &input, &options);
        assert!(result.is_some());
        let reason = result
            .unwrap()
            .hook_specific_output
            .permission_decision_reason
            .unwrap();
        assert!(reason.contains("See guidelines"));
    }

    #[test]
    fn test_deny_rust_allow_case_insensitive_extension() {
        let input = make_tool_input("src/main.RS", "#[allow(dead_code)]");
        let result = deny_rust_allow("Edit", &input, &default_options());
        assert!(result.is_some()); // Should match .RS as well
    }

    #[test]
    fn test_deny_rust_allow_allows_normal_code() {
        let input = make_tool_input("src/main.rs", "fn foo() { println!(\"hello\"); }");
        let result = deny_rust_allow("Edit", &input, &default_options());
        assert!(result.is_none());
    }
}
