use regex::Regex;
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
const DESTRUCTIVE_PATTERNS: &[(&str, &str); 1] = &[(
    r"\|\s*(move|move-item)\b",
    "piped to move/move-item",
)];

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
        "permission-request" => block_rm(&cmd).or_else(|| confirm_destructive_find(&cmd)),
        _ => {
            eprintln!("Unknown hook: {hook_name}");
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
        eprintln!("  permission-request    - Check and handle permission requests");
        std::process::exit(1);
    }

    let hook_name = &args[1];

    if let Err(e) = run_hook(hook_name) {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }

    std::process::exit(0);
}
