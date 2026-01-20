use agent_hooks::{
    PackageManagerCheckResult, RustAllowCheckResult, check_dangerous_path_command,
    check_destructive_find, check_package_manager, check_rust_allow_attributes, is_rm_command,
    is_rust_file,
};
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
    let dangerous_paths = c.string_flag("dangerous-paths").ok();

    if !block_rm && !confirm_destructive_find && dangerous_paths.is_none() {
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

    // Check for dangerous path operations (rm/trash/mv on dangerous paths)
    if let Some(ref paths_str) = dangerous_paths {
        let paths: Vec<&str> = paths_str.split(',').map(str::trim).collect();
        if let Some(check) = check_dangerous_path_command(cmd, &paths) {
            output_hook_result(&HookOutput {
                hook_specific_output: HookSpecificOutput {
                    hook_event_name: HookEventName::PermissionRequest,
                    decision: None,
                    permission_decision: Some(PermissionDecision::Ask),
                    permission_decision_reason: Some(format!(
                        "Dangerous path operation detected: {} command targeting protected path '{}'. \
                         Please confirm this operation.",
                        check.command_type, check.matched_path
                    )),
                },
            });
            return;
        }
    }

    // Check for destructive find command
    if confirm_destructive_find && let Some(description) = check_destructive_find(cmd) {
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

/// Handle package manager mismatch checks for Bash commands.
/// Returns `true` if output was produced and the caller should return early.
fn handle_package_manager_check(cmd: &str) -> bool {
    let cwd = std::env::current_dir().unwrap_or_default();
    match check_package_manager(cmd, &cwd) {
        PackageManagerCheckResult::Mismatch {
            command_pm,
            expected_pm,
        } => {
            output_hook_result(&HookOutput {
                hook_specific_output: HookSpecificOutput {
                    hook_event_name: HookEventName::PreToolUse,
                    decision: None,
                    permission_decision: Some(PermissionDecision::Deny),
                    permission_decision_reason: Some(format!(
                        "Package manager mismatch: This project uses {} (detected {}), \
                         but you are trying to use {}. Please use {} instead.",
                        expected_pm.name(),
                        expected_pm.lock_files()[0],
                        command_pm.name(),
                        expected_pm.name()
                    )),
                },
            });
            true
        }
        // Multiple lock files or no mismatch: don't intervene
        _ => false,
    }
}

/// Build denial reason for Rust allow/expect attributes.
fn build_rust_allow_denial_reason(
    check_result: RustAllowCheckResult,
    expect_flag: bool,
    additional_context: Option<&str>,
) -> Option<String> {
    let base_msg = if expect_flag {
        match check_result {
            RustAllowCheckResult::HasAllow | RustAllowCheckResult::HasBoth => Some(
                "Adding #[allow(...)] or #![allow(...)] attributes is not permitted. \
                 Use #[expect(...)] instead, which will warn when the lint is no longer triggered.",
            ),
            _ => None,
        }
    } else {
        match check_result {
            RustAllowCheckResult::Ok => None,
            RustAllowCheckResult::HasBoth => Some(
                "Adding #[allow(...)] or #[expect(...)] attributes is not permitted. \
                 Fix the underlying issue instead of suppressing the warning.",
            ),
            RustAllowCheckResult::HasAllow => Some(
                "Adding #[allow(...)] or #![allow(...)] attributes is not permitted. \
                 Fix the underlying issue instead of suppressing the warning.",
            ),
            RustAllowCheckResult::HasExpect => Some(
                "Adding #[expect(...)] or #![expect(...)] attributes is not permitted. \
                 Fix the underlying issue instead of suppressing the warning.",
            ),
        }
    };

    base_msg.map(|msg| {
        let mut result = msg.to_string();
        if let Some(ctx) = additional_context {
            result.push(' ');
            result.push_str(ctx);
        }
        result
    })
}

fn pre_tool_use_action(c: &Context) {
    let deny_rust_allow_enabled = c.bool_flag("deny-rust-allow");
    let check_package_manager_enabled = c.bool_flag("check-package-manager");

    if !deny_rust_allow_enabled && !check_package_manager_enabled {
        return;
    }

    let Ok(data) = read_hook_input() else {
        return;
    };

    let Some(ref tool_name) = data.tool_name else {
        return;
    };

    // Package manager check for Bash commands
    if check_package_manager_enabled && matches!(tool_name, ToolName::Bash) {
        let cmd = data
            .tool_input
            .as_ref()
            .and_then(|ti| ti.command.as_deref())
            .unwrap_or_default();

        if !cmd.is_empty() && handle_package_manager_check(cmd) {
            return;
        }
    }

    // Only check Edit and Write tools for Rust allow attributes
    if !matches!(tool_name, ToolName::Edit | ToolName::Write) {
        return;
    }

    if !deny_rust_allow_enabled {
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

    if let Some(reason) =
        build_rust_allow_denial_reason(check_result, expect_flag, additional_context.as_deref())
    {
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
                .flag(
                    Flag::new("dangerous-paths", FlagType::String)
                        .description("Comma-separated list of dangerous paths to protect from rm/trash/mv"),
                )
                .action(permission_request_action),
        )
        .command(
            Command::new("pre-tool-use")
                .description("Handle pre-tool-use checks for Edit/Write/Bash tools")
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
                .flag(
                    Flag::new("check-package-manager", FlagType::Bool)
                        .description("Check for package manager mismatch (e.g., using npm when pnpm-lock.yaml exists)"),
                )
                .action(pre_tool_use_action),
        );

    app.run(args);
}
