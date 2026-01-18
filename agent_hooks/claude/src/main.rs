use agent_hooks::{check_destructive_find, check_rust_allow_attributes, is_rm_command, is_rust_file, RustAllowCheckResult};
use seahorse::{App, Command, Context, Flag, FlagType};
use serde::{Deserialize, Serialize};
use std::io::{self, Read};

// ============================================================================
// Claude Code specific types
// ============================================================================

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
    pub new_string: Option<String>,
    pub content: Option<String>,
    pub file_path: Option<String>,
}

/// Hook event names for Claude Code output
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

/// Permission decision types
#[derive(Debug, Clone, Copy, Serialize)]
#[non_exhaustive]
#[serde(rename_all = "lowercase")]
pub enum PermissionDecision {
    Ask,
    Allow,
    Deny,
}

/// Output to be printed as JSON to stdout for Claude Code
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

    #[serde(skip_serializing_if = "Option::is_none")]
    pub decision: Option<Decision>,

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
// Command handlers
// ============================================================================

fn permission_request_action(c: &Context) {
    let block_rm = c.bool_flag("block-rm");
    let confirm_destructive_find = c.bool_flag("confirm-destructive-find");

    if !block_rm && !confirm_destructive_find {
        return;
    }

    let Ok(data) = read_hook_input() else {
        return;
    };

    // Only handle Bash commands
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

    // Check for rm command
    if block_rm && is_rm_command(cmd) {
        output_hook_result(&HookOutput {
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
        return;
    }

    // Check for destructive find command
    if confirm_destructive_find {
        if let Some(description) = check_destructive_find(cmd) {
            output_hook_result(&HookOutput {
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
}

fn pre_tool_use_action(c: &Context) {
    let deny_rust_allow_enabled = c.bool_flag("deny-rust-allow");

    if !deny_rust_allow_enabled {
        return;
    }

    let Ok(data) = read_hook_input() else {
        return;
    };

    // Only check Edit and Write tools
    let Some(ref tool_name) = data.tool_name else {
        return;
    };
    if !matches!(tool_name, ToolName::Edit | ToolName::Write) {
        return;
    }

    let Some(ref tool_input) = data.tool_input else {
        return;
    };

    // Check if this is a Rust file
    let file_path = tool_input.file_path.as_deref().unwrap_or_default();
    if !is_rust_file(file_path) {
        return;
    }

    // Get the content being written/edited
    let content = tool_input
        .new_string
        .as_deref()
        .or(tool_input.content.as_deref())
        .unwrap_or_default();

    if content.is_empty() {
        return;
    }

    let expect_flag = c.bool_flag("expect");
    let additional_context = c.string_flag("additional-context").ok();

    let check_result = check_rust_allow_attributes(content);

    let denial_reason = if expect_flag {
        // --expect: only deny #[allow], allow #[expect]
        match check_result {
            RustAllowCheckResult::HasAllow | RustAllowCheckResult::HasBoth => {
                let mut msg = "Adding #[allow(...)] or #![allow(...)] attributes is not permitted. \
                               Use #[expect(...)] instead, which will warn when the lint is no longer triggered."
                    .to_string();
                if let Some(ref ctx) = additional_context {
                    msg.push(' ');
                    msg.push_str(ctx);
                }
                Some(msg)
            }
            _ => None,
        }
    } else {
        // no --expect: deny both #[allow] and #[expect]
        match check_result {
            RustAllowCheckResult::Ok => None,
            RustAllowCheckResult::HasBoth => {
                let mut msg = "Adding #[allow(...)] or #[expect(...)] attributes is not permitted. \
                               Fix the underlying issue instead of suppressing the warning."
                    .to_string();
                if let Some(ref ctx) = additional_context {
                    msg.push(' ');
                    msg.push_str(ctx);
                }
                Some(msg)
            }
            RustAllowCheckResult::HasAllow => {
                let mut msg = "Adding #[allow(...)] or #![allow(...)] attributes is not permitted. \
                               Fix the underlying issue instead of suppressing the warning."
                    .to_string();
                if let Some(ref ctx) = additional_context {
                    msg.push(' ');
                    msg.push_str(ctx);
                }
                Some(msg)
            }
            RustAllowCheckResult::HasExpect => {
                let mut msg = "Adding #[expect(...)] or #![expect(...)] attributes is not permitted. \
                               Fix the underlying issue instead of suppressing the warning."
                    .to_string();
                if let Some(ref ctx) = additional_context {
                    msg.push(' ');
                    msg.push_str(ctx);
                }
                Some(msg)
            }
        }
    };

    if let Some(reason) = denial_reason {
        output_hook_result(&HookOutput {
            hook_specific_output: HookSpecificOutput {
                hook_event_name: HookEventName::PreToolUse,
                decision: None,
                permission_decision: Some(PermissionDecision::Deny),
                permission_decision_reason: Some(reason),
            },
        });
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
                        .description(
                            "With --deny-rust-allow: additional context message to append to the denial reason",
                        ),
                )
                .action(pre_tool_use_action),
        );

    app.run(args);
}
