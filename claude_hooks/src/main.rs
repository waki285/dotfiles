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
#[non_exhaustive]
pub enum HookEventName {
    PermissionRequest,
    PreToolUse,
}

/// Behavior for permission decisions
#[derive(Debug, Clone, Copy, Serialize)]
#[non_exhaustive]
#[serde(rename_all = "lowercase")]
pub enum DecisionBehavior {
    Deny,
    Allow,
}

/// Permission decision types for ask behavior
#[derive(Debug, Clone, Copy, Serialize)]
#[non_exhaustive]
#[serde(rename_all = "lowercase")]
pub enum PermissionDecision {
    Ask,
    Allow,
    Deny,
}

/// Tool names that Claude Code can invoke
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[non_exhaustive]
pub enum ToolName {
    Task,
    Bash,
    Glob,
    Grep,
    Read,
    Edit,
    Write,
    WebFetch,
    WebSearch,
    #[serde(untagged)]
    Unknown(String),
}

// ============================================================================
// Input structures
// ============================================================================

/// Input received from Claude Code hooks via stdin
#[derive(Debug, Deserialize)]
#[non_exhaustive]
pub struct HookInput {
    pub tool_name: Option<ToolName>,
    pub tool_input: Option<ToolInput>,
}

#[derive(Debug, Deserialize)]
#[non_exhaustive]
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
#[non_exhaustive]
#[serde(rename_all = "camelCase")]
pub struct HookOutput {
    pub hook_specific_output: HookSpecificOutput,
}

#[derive(Debug, Serialize)]
#[non_exhaustive]
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
#[non_exhaustive]
pub struct Decision {
    pub behavior: DecisionBehavior,
    pub message: String,
}

// ============================================================================
// Helper functions
// ============================================================================

#[inline]
fn read_hook_input() -> io::Result<HookInput> {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input)?;
    serde_json::from_str(&input).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

#[inline]
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
#[inline]
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
    tool_name: &ToolName,
    tool_input: &ToolInput,
    options: &DenyRustAllowOptions,
) -> Option<HookOutput> {
    // Only check Edit and Write tools
    if !matches!(tool_name, ToolName::Edit | ToolName::Write) {
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

fn permission_request_action(c: &Context) {
    // Check if any module is enabled
    let block_rm_enabled = c.bool_flag("block-rm");
    let confirm_destructive_find_enabled = c.bool_flag("confirm-destructive-find");

    // If no module is enabled, do nothing
    if !block_rm_enabled && !confirm_destructive_find_enabled {
        return;
    }

    let Ok(data) = read_hook_input() else {
        return;
    };

    let Some(ToolName::Bash) = data.tool_name else {
        return;
    };

    let cmd = data
        .tool_input
        .as_ref()
        .and_then(|ti| ti.command.as_deref())
        .unwrap_or_default();

    if cmd.is_empty() {
        return;
    }

    // Run enabled modules
    let output = if block_rm_enabled {
        block_rm(cmd)
    } else {
        None
    }
    .or_else(|| {
        if confirm_destructive_find_enabled {
            confirm_destructive_find(cmd)
        } else {
            None
        }
    });

    if let Some(output) = output {
        output_hook_result(&output);
    }
}

fn pre_tool_use_action(c: &Context) {
    // Check if any module is enabled
    let deny_rust_allow_enabled = c.bool_flag("deny-rust-allow");

    // If no module is enabled, do nothing
    if !deny_rust_allow_enabled {
        return;
    }

    let Ok(data) = read_hook_input() else {
        return;
    };

    let Some(ref tool_name) = data.tool_name else {
        return;
    };

    let Some(tool_input) = data.tool_input.as_ref() else {
        return;
    };

    // Run enabled modules
    if deny_rust_allow_enabled {
        if !matches!(tool_name, ToolName::Edit | ToolName::Write) {
            return;
        }

        // Parse flags for deny-rust-allow
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
                .description("Handle permission requests for Bash commands")
                .flag(
                    Flag::new("block-rm", FlagType::Bool)
                        .description("Block rm command and suggest using trash instead"),
                )
                .flag(
                    Flag::new("confirm-destructive-find", FlagType::Bool)
                        .description("Ask for confirmation on destructive find commands"),
                )
                .action(permission_request_action),
        )
        .command(
            Command::new("pre-tool-use")
                .description("Handle pre-tool-use checks for Edit/Write tools")
                .flag(
                    Flag::new("deny-rust-allow", FlagType::Bool)
                        .description("Deny #[allow(...)] attributes in Rust files"),
                )
                .flag(
                    Flag::new("expect", FlagType::Bool)
                        .description("With --deny-rust-allow: suggest #[expect(...)] instead of denying both"),
                )
                .flag(
                    Flag::new("additional-context", FlagType::String)
                        .description("With --deny-rust-allow: additional context message to append to the denial reason"),
                )
                .action(pre_tool_use_action),
        );

    app.run(args);
}

#[cfg(test)]
mod tests;
