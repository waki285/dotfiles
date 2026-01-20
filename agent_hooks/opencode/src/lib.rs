//! NAPI bindings for `agent_hooks`, used by `OpenCode`.
//!
//! These bindings expose simple check functions that can be called directly
//! from TypeScript/JavaScript without JSON serialization overhead.
#![expect(clippy::needless_pass_by_value)]

use agent_hooks::{
    PackageManagerCheckResult, RustAllowCheckResult, check_dangerous_path_command,
    check_destructive_find, check_package_manager, check_rust_allow_attributes, is_rm_command,
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

/// Result of checking for dangerous path operations.
#[napi(object)]
pub struct DangerousPathResult {
    /// The dangerous path that was matched.
    pub matched_path: String,
    /// The command type (rm, trash, mv).
    pub command_type: String,
}

/// Check if a bash command targets dangerous paths with rm/trash/mv.
///
/// Returns the matched dangerous path and command type if detected, or `null` if safe.
#[napi(js_name = "checkDangerousPathCommand")]
pub fn check_dangerous_path_command_js(
    cmd: String,
    dangerous_paths: Vec<String>,
) -> Option<DangerousPathResult> {
    let paths: Vec<&str> = dangerous_paths.iter().map(String::as_str).collect();
    check_dangerous_path_command(&cmd, &paths).map(|check| DangerousPathResult {
        matched_path: check.matched_path,
        command_type: check.command_type,
    })
}

/// Result of checking for package manager mismatch.
#[napi(string_enum)]
pub enum PackageManagerCheck {
    /// No package manager command detected or no lock file found.
    Ok,
    /// Command matches the lock file's package manager.
    Matching,
    /// Command uses a different package manager than the lock file indicates (should deny).
    Mismatch,
    /// Multiple lock files exist (should ask).
    Ambiguous,
}

/// Detailed result of checking for package manager mismatch.
#[napi(object)]
pub struct PackageManagerCheckResultJs {
    /// The check result type.
    pub result: PackageManagerCheck,
    /// The package manager being used in the command (if detected).
    pub command_pm: Option<String>,
    /// The expected package manager based on lock file (for Mismatch).
    pub expected_pm: Option<String>,
    /// Lock files detected (for Mismatch/Ambiguous).
    pub detected_lock_files: Option<Vec<String>>,
}

/// Check if a bash command uses a mismatched package manager.
///
/// Searches for lock files starting from `start_dir` and going up to parent directories.
#[napi(js_name = "checkPackageManager")]
#[must_use]
pub fn check_package_manager_js(cmd: String, start_dir: String) -> PackageManagerCheckResultJs {
    let path = std::path::Path::new(&start_dir);
    match check_package_manager(&cmd, path) {
        PackageManagerCheckResult::Ok => PackageManagerCheckResultJs {
            result: PackageManagerCheck::Ok,
            command_pm: None,
            expected_pm: None,
            detected_lock_files: None,
        },
        PackageManagerCheckResult::Matching => PackageManagerCheckResultJs {
            result: PackageManagerCheck::Matching,
            command_pm: None,
            expected_pm: None,
            detected_lock_files: None,
        },
        PackageManagerCheckResult::Mismatch {
            command_pm,
            expected_pm,
        } => PackageManagerCheckResultJs {
            result: PackageManagerCheck::Mismatch,
            command_pm: Some(command_pm.name().to_string()),
            expected_pm: Some(expected_pm.name().to_string()),
            detected_lock_files: Some(
                expected_pm
                    .lock_files()
                    .iter()
                    .map(|s| (*s).to_string())
                    .collect(),
            ),
        },
        PackageManagerCheckResult::Ambiguous {
            command_pm,
            detected_pms,
        } => PackageManagerCheckResultJs {
            result: PackageManagerCheck::Ambiguous,
            command_pm: Some(command_pm.name().to_string()),
            expected_pm: None,
            detected_lock_files: Some(
                detected_pms
                    .iter()
                    .flat_map(|pm| pm.lock_files().iter().map(|s| (*s).to_string()))
                    .collect(),
            ),
        },
    }
}
