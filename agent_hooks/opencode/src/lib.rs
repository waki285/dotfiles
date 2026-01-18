//! NAPI bindings for `agent_hooks`, used by `OpenCode`.
//!
//! These bindings expose simple check functions that can be called directly
//! from TypeScript/JavaScript without JSON serialization overhead.
#![expect(clippy::needless_pass_by_value)]

use agent_hooks::{
    RustAllowCheckResult, check_destructive_find, check_rust_allow_attributes, is_rm_command,
    is_rust_file,
};
use napi_derive::napi;

/// Check if a command contains an rm (or equivalent) command.
///
/// Returns `true` if the command should be blocked.
#[napi(js_name = "isRmCommand")]
#[must_use]
pub fn is_rm_command_js(cmd: String) -> bool {
    is_rm_command(&cmd)
}

/// Check if a command is a destructive find command.
///
/// Returns the description of the destructive pattern if found, or `null` if safe.
#[napi(js_name = "checkDestructiveFind")]
pub fn check_destructive_find_js(cmd: String) -> Option<String> {
    check_destructive_find(&cmd).map(String::from)
}

/// Check if a file path is a Rust file.
#[napi(js_name = "isRustFile")]
#[must_use]
pub fn is_rust_file_js(file_path: String) -> bool {
    is_rust_file(&file_path)
}

/// Result of checking for Rust allow/expect attributes.
#[napi(string_enum)]
pub enum RustAllowCheck {
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
#[napi(js_name = "checkRustAllowAttributes")]
#[must_use]
pub fn check_rust_allow_attributes_js(content: String) -> RustAllowCheck {
    match check_rust_allow_attributes(&content) {
        RustAllowCheckResult::Ok => RustAllowCheck::Ok,
        RustAllowCheckResult::HasAllow => RustAllowCheck::HasAllow,
        RustAllowCheckResult::HasExpect => RustAllowCheck::HasExpect,
        RustAllowCheckResult::HasBoth => RustAllowCheck::HasBoth,
    }
}
