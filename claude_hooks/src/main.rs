use regex::Regex;
use serde::{Deserialize, Serialize};
use std::io::{self, Read};

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
}

// ============================================================================
// Output structures
// ============================================================================

/// Output to be printed as JSON to stdout
#[derive(Debug, Serialize)]
pub struct HookOutput {
    #[serde(rename = "hookSpecificOutput")]
    pub hook_specific_output: HookSpecificOutput,
}

#[derive(Debug, Serialize)]
pub struct HookSpecificOutput {
    #[serde(rename = "hookEventName")]
    pub hook_event_name: String,

    /// Used for deny behavior (block-rm)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decision: Option<Decision>,

    /// Used for ask behavior (confirm-destructive-find)
    #[serde(rename = "permissionDecision", skip_serializing_if = "Option::is_none")]
    pub permission_decision: Option<String>,

    #[serde(
        rename = "permissionDecisionReason",
        skip_serializing_if = "Option::is_none"
    )]
    pub permission_decision_reason: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct Decision {
    pub behavior: String,
    pub message: String,
}

// ============================================================================
// Hook implementations
// ============================================================================

/// Block rm command and suggest using trash instead
fn block_rm(cmd: &str) -> Option<HookOutput> {
    let rm_pattern =
        Regex::new(r"(^|[;&|()]\s*)(sudo\s+)?(command\s+)?(\\)?(\S*/)?rm(\s|$)").unwrap();

    if rm_pattern.is_match(cmd) {
        return Some(HookOutput {
            hook_specific_output: HookSpecificOutput {
                hook_event_name: "PermissionRequest".to_string(),
                decision: Some(Decision {
                    behavior: "deny".to_string(),
                    message: "rm is forbidden. Use trash command to delete files. Example: trash <path...>".to_string(),
                }),
                permission_decision: None,
                permission_decision_reason: None,
            },
        });
    }

    None
}

/// Confirm destructive find commands
fn confirm_destructive_find(cmd: &str) -> Option<HookOutput> {
    // First check if this is a find command
    let find_check = Regex::new(r"(^|[;&|()]\s*)find\s").unwrap();
    if !find_check.is_match(cmd) {
        return None;
    }

    // Destructive patterns with descriptions
    let destructive_patterns: &[(&str, &str)] = &[
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

    for (pattern, description) in destructive_patterns {
        let re = Regex::new(&format!("(?i){}", pattern)).unwrap();
        if re.is_match(cmd) {
            return Some(HookOutput {
                hook_specific_output: HookSpecificOutput {
                    hook_event_name: "PermissionRequest".to_string(),
                    decision: None,
                    permission_decision: Some("ask".to_string()),
                    permission_decision_reason: Some(format!(
                        "Destructive find command detected: {}. \
                         This operation may delete or modify files. Please confirm.",
                        description
                    )),
                },
            });
        }
    }

    None
}

fn run_hook(hook_name: &str) -> io::Result<()> {
    // Read JSON from stdin
    let mut input = String::new();
    io::stdin().read_to_string(&mut input)?;

    let data: HookInput = match serde_json::from_str(&input) {
        Ok(d) => d,
        Err(_) => return Ok(()), // Invalid JSON, just exit
    };

    // Check if this is a Bash command
    let tool_name = data.tool_name.unwrap_or_default();
    if tool_name != "Bash" {
        return Ok(());
    }

    let cmd = data
        .tool_input
        .and_then(|ti| ti.command)
        .unwrap_or_default();

    if cmd.is_empty() {
        return Ok(());
    }

    // Run the appropriate hook
    let output = match hook_name {
        "block-rm" => block_rm(&cmd),
        "confirm-destructive-find" => confirm_destructive_find(&cmd),
        _ => {
            eprintln!("Unknown hook: {}", hook_name);
            return Ok(());
        }
    };

    // Print output if any
    if let Some(out) = output {
        println!("{}", serde_json::to_string(&out)?);
    }

    Ok(())
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: {} <hook-name>", args[0]);
        eprintln!("Available hooks:");
        eprintln!("  block-rm                    - Block rm command, suggest trash");
        eprintln!("  confirm-destructive-find    - Confirm destructive find commands");
        std::process::exit(1);
    }

    let hook_name = &args[1];

    if let Err(e) = run_hook(hook_name) {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
