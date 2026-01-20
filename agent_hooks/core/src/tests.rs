//! Unit tests for `agent_hooks` core

use super::*;

const LOCK_FILES: &[&str] = &[
    "package-lock.json",
    "pnpm-lock.yaml",
    "yarn.lock",
    "bun.lockb",
    "bun.lock",
];

fn cleanup_lock_files(dir: &std::path::Path) {
    for file in LOCK_FILES {
        let _ = std::fs::remove_file(dir.join(file));
    }
}

// -------------------------------------------------------------------------
// is_in_comment_or_string tests
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
    let content = "let s = \"#[allow(dead_code)]\";";
    assert!(is_in_comment_or_string(content, 9));
}

#[test]
fn test_is_in_comment_or_string_after_comment() {
    let content = "// comment\n#[allow(dead_code)]";
    assert!(!is_in_comment_or_string(content, 11));
}

// -------------------------------------------------------------------------
// is_rm_command tests
// -------------------------------------------------------------------------

#[test]
fn test_is_rm_command_simple() {
    assert!(is_rm_command("rm file.txt"));
}

#[test]
fn test_is_rm_command_with_flags() {
    assert!(is_rm_command("rm -rf /tmp/test"));
}

#[test]
fn test_is_rm_command_with_sudo() {
    assert!(is_rm_command("sudo rm -rf /"));
}

#[test]
fn test_is_rm_command_in_pipeline() {
    assert!(is_rm_command("echo test && rm file.txt"));
}

#[test]
fn test_is_rm_command_allows_other_commands() {
    assert!(!is_rm_command("ls -la"));
    assert!(!is_rm_command("trash file.txt"));
}

#[test]
fn test_is_rm_command_allows_grep_rm() {
    assert!(!is_rm_command("grep -r 'pattern' ."));
    assert!(!is_rm_command("rma -rm"));
}

#[test]
fn test_is_rm_command_xargs_rm() {
    assert!(is_rm_command("ls | xargs rm"));
    assert!(is_rm_command("cat files.txt | xargs rm -f"));
    assert!(is_rm_command("find . -name '*.tmp' | xargs rm"));
}

#[test]
fn test_is_rm_command_xargs_rmdir() {
    assert!(is_rm_command("ls | xargs rmdir"));
    assert!(is_rm_command("cat dirs.txt | xargs rmdir"));
}

#[test]
fn test_is_rm_command_xargs_with_sudo() {
    assert!(is_rm_command("ls | xargs sudo rm"));
    assert!(is_rm_command("find . | sudo xargs rm"));
}

// -------------------------------------------------------------------------
// check_destructive_find tests (Unix only)
// -------------------------------------------------------------------------

#[cfg(not(windows))]
#[test]
fn test_check_destructive_find_delete() {
    let result = check_destructive_find("find . -name '*.tmp' -delete");
    assert!(result.is_some());
    assert_eq!(result.unwrap(), "find with -delete option");
}

#[cfg(not(windows))]
#[test]
fn test_check_destructive_find_exec_rm() {
    let result = check_destructive_find("find . -exec rm {} \\;");
    assert!(result.is_some());
}

#[cfg(not(windows))]
#[test]
fn test_check_destructive_find_xargs_rm() {
    let result = check_destructive_find("find . -name '*.tmp' | xargs rm");
    assert!(result.is_some());
}

#[cfg(not(windows))]
#[test]
fn test_check_destructive_find_safe() {
    assert!(check_destructive_find("find . -name '*.rs'").is_none());
    assert!(check_destructive_find("find . -type f -print").is_none());
}

// -------------------------------------------------------------------------
// check_destructive_find tests (Windows only)
// -------------------------------------------------------------------------

#[cfg(windows)]
#[test]
fn test_check_destructive_find_piped_move() {
    let result = check_destructive_find("dir | move-item");
    assert!(result.is_some());
}

#[cfg(windows)]
#[test]
fn test_check_destructive_find_safe() {
    assert!(check_destructive_find("dir /s").is_none());
    assert!(check_destructive_find("Get-ChildItem").is_none());
}

// -------------------------------------------------------------------------
// check_rust_allow_attributes tests
// -------------------------------------------------------------------------

#[test]
fn test_check_rust_allow_detects_allow() {
    let result = check_rust_allow_attributes("#[allow(dead_code)]\nfn foo() {}");
    assert_eq!(result, RustAllowCheckResult::HasAllow);
}

#[test]
fn test_check_rust_allow_detects_inner_allow() {
    let result = check_rust_allow_attributes("#![allow(unused)]");
    assert_eq!(result, RustAllowCheckResult::HasAllow);
}

#[test]
fn test_check_rust_allow_detects_expect() {
    let result = check_rust_allow_attributes("#[expect(dead_code)]");
    assert_eq!(result, RustAllowCheckResult::HasExpect);
}

#[test]
fn test_check_rust_allow_detects_both() {
    let result = check_rust_allow_attributes("#[allow(dead_code)]\n#[expect(unused)]");
    assert_eq!(result, RustAllowCheckResult::HasBoth);
}

#[test]
fn test_check_rust_allow_ignores_comments() {
    let result = check_rust_allow_attributes("// #[allow(dead_code)]\nfn foo() {}");
    assert_eq!(result, RustAllowCheckResult::Ok);
}

#[test]
fn test_check_rust_allow_ignores_string_literals() {
    let result = check_rust_allow_attributes("let s = \"#[allow(dead_code)]\";");
    assert_eq!(result, RustAllowCheckResult::Ok);
}

#[test]
fn test_check_rust_allow_allows_normal_code() {
    let result = check_rust_allow_attributes("fn foo() { println!(\"hello\"); }");
    assert_eq!(result, RustAllowCheckResult::Ok);
}

#[test]
fn test_check_rust_allow_after_comment() {
    let result = check_rust_allow_attributes("// comment\n#[allow(dead_code)]");
    assert_eq!(result, RustAllowCheckResult::HasAllow);
}

// -------------------------------------------------------------------------
// is_rust_file tests
// -------------------------------------------------------------------------

#[test]
fn test_is_rust_file_rs() {
    assert!(is_rust_file("src/main.rs"));
    assert!(is_rust_file("lib.rs"));
    assert!(is_rust_file("/path/to/file.RS"));
}

#[test]
fn test_is_rust_file_not_rs() {
    assert!(!is_rust_file("README.md"));
    assert!(!is_rust_file("Cargo.toml"));
    assert!(!is_rust_file("script.py"));
}

// -------------------------------------------------------------------------
// check_dangerous_path_command tests
// -------------------------------------------------------------------------

#[test]
fn test_dangerous_path_rm_home_exact() {
    // "~/" pattern should match exact home directory
    let dangerous = &["~/"];
    let result = check_dangerous_path_command("rm -rf ~/", dangerous);
    assert!(result.is_some());
    let check = result.unwrap();
    assert_eq!(check.command_type, "rm");
    assert_eq!(check.matched_path, "~/");
}

#[test]
fn test_dangerous_path_rm_home_wildcard() {
    // "~/" pattern should match wildcards directly under home
    let dangerous = &["~/"];
    let result = check_dangerous_path_command("rm -rf ~/*", dangerous);
    assert!(result.is_some());
    assert_eq!(result.unwrap().matched_path, "~/");
}

#[test]
fn test_dangerous_path_rm_home_hidden_wildcard() {
    // "~/" pattern should match hidden file wildcards
    let dangerous = &["~/"];
    let result = check_dangerous_path_command("rm -rf ~/.*", dangerous);
    assert!(result.is_some());
}

#[test]
fn test_dangerous_path_rm_home_subdir_allowed() {
    // "~/" pattern should NOT match specific files/directories under home
    let dangerous = &["~/"];
    let result = check_dangerous_path_command("rm -rf ~/Documents", dangerous);
    assert!(result.is_none());
}

#[test]
fn test_dangerous_path_rm_home_file_allowed() {
    // "~/" pattern should NOT match specific files under home
    let dangerous = &["~/"];
    let result = check_dangerous_path_command("rm ~/file.txt", dangerous);
    assert!(result.is_none());
}

#[test]
fn test_dangerous_path_rm_subdir_wildcard_allowed() {
    // "~/" pattern should NOT match wildcards in subdirectories
    let dangerous = &["~/"];
    let result = check_dangerous_path_command("rm -rf ~/Downloads/*", dangerous);
    assert!(result.is_none());
}

#[test]
fn test_dangerous_path_trash_home_wildcard() {
    let dangerous = &["~/"];
    let result = check_dangerous_path_command("trash ~/*", dangerous);
    assert!(result.is_some());
    let check = result.unwrap();
    assert_eq!(check.command_type, "trash");
}

#[test]
fn test_dangerous_path_mv_home() {
    let dangerous = &["~/"];
    let result = check_dangerous_path_command("mv ~/ /tmp/backup", dangerous);
    assert!(result.is_some());
    let check = result.unwrap();
    assert_eq!(check.command_type, "mv");
}

#[test]
fn test_dangerous_path_exact_path_match() {
    // Exact path (without trailing /) should match that path and children
    let dangerous = &["/etc/nginx"];
    let result = check_dangerous_path_command("rm -rf /etc/nginx", dangerous);
    assert!(result.is_some());
}

#[test]
fn test_dangerous_path_exact_path_child_match() {
    let dangerous = &["/etc/nginx"];
    let result = check_dangerous_path_command("rm /etc/nginx/nginx.conf", dangerous);
    assert!(result.is_some());
}

#[test]
fn test_dangerous_path_safe_location() {
    let dangerous = &["~/", "/etc"];
    let result = check_dangerous_path_command("rm -rf /tmp/test", dangerous);
    assert!(result.is_none());
}

#[test]
fn test_dangerous_path_with_sudo() {
    let dangerous = &["~/"];
    let result = check_dangerous_path_command("sudo rm -rf ~/*", dangerous);
    assert!(result.is_some());
}

#[test]
fn test_dangerous_path_chained_commands() {
    let dangerous = &["~/"];
    let result = check_dangerous_path_command("echo test; rm ~/*", dangerous);
    assert!(result.is_some());
}

// -------------------------------------------------------------------------
// detect_package_manager_command tests
// -------------------------------------------------------------------------

#[test]
fn test_detect_pm_npm_install() {
    assert_eq!(
        detect_package_manager_command("npm install"),
        Some(PackageManager::Npm)
    );
    assert_eq!(
        detect_package_manager_command("npm i"),
        Some(PackageManager::Npm)
    );
    assert_eq!(
        detect_package_manager_command("npm add lodash"),
        Some(PackageManager::Npm)
    );
    assert_eq!(
        detect_package_manager_command("npm ci"),
        Some(PackageManager::Npm)
    );
}

#[test]
fn test_detect_pm_pnpm_install() {
    assert_eq!(
        detect_package_manager_command("pnpm install"),
        Some(PackageManager::Pnpm)
    );
    assert_eq!(
        detect_package_manager_command("pnpm add lodash"),
        Some(PackageManager::Pnpm)
    );
    assert_eq!(
        detect_package_manager_command("pnpm remove lodash"),
        Some(PackageManager::Pnpm)
    );
}

#[test]
fn test_detect_pm_yarn_install() {
    assert_eq!(
        detect_package_manager_command("yarn install"),
        Some(PackageManager::Yarn)
    );
    assert_eq!(
        detect_package_manager_command("yarn add lodash"),
        Some(PackageManager::Yarn)
    );
}

#[test]
fn test_detect_pm_bun_install() {
    assert_eq!(
        detect_package_manager_command("bun install"),
        Some(PackageManager::Bun)
    );
    assert_eq!(
        detect_package_manager_command("bun add lodash"),
        Some(PackageManager::Bun)
    );
}

#[test]
fn test_detect_pm_no_match() {
    assert_eq!(detect_package_manager_command("npm run build"), None);
    assert_eq!(detect_package_manager_command("npm start"), None);
    assert_eq!(detect_package_manager_command("pnpm run dev"), None);
    assert_eq!(detect_package_manager_command("yarn build"), None);
    assert_eq!(detect_package_manager_command("bun run script.ts"), None);
    assert_eq!(detect_package_manager_command("ls -la"), None);
}

#[test]
fn test_detect_pm_with_sudo() {
    assert_eq!(
        detect_package_manager_command("sudo npm install"),
        Some(PackageManager::Npm)
    );
}

#[test]
fn test_detect_pm_chained_commands() {
    assert_eq!(
        detect_package_manager_command("cd /app && npm install"),
        Some(PackageManager::Npm)
    );
    assert_eq!(
        detect_package_manager_command("echo test; pnpm add lodash"),
        Some(PackageManager::Pnpm)
    );
}

// -------------------------------------------------------------------------
// check_package_manager tests (using temp directories)
// -------------------------------------------------------------------------

#[test]
fn test_check_pm_no_lock_file() {
    let temp_dir = std::env::temp_dir().join("agent_hooks_test_no_lock");
    let _ = std::fs::create_dir_all(&temp_dir);

    cleanup_lock_files(&temp_dir);

    let result = check_package_manager("npm install", &temp_dir);
    assert_eq!(result, PackageManagerCheckResult::Ok);

    let _ = std::fs::remove_dir(&temp_dir);
}

#[test]
fn test_check_pm_matching() {
    let temp_dir = std::env::temp_dir().join("agent_hooks_test_matching");
    let _ = std::fs::create_dir_all(&temp_dir);

    cleanup_lock_files(&temp_dir);

    std::fs::write(temp_dir.join("pnpm-lock.yaml"), "").unwrap();

    let result = check_package_manager("pnpm install", &temp_dir);
    assert_eq!(result, PackageManagerCheckResult::Matching);

    let _ = std::fs::remove_file(temp_dir.join("pnpm-lock.yaml"));
    let _ = std::fs::remove_dir(&temp_dir);
}

#[test]
fn test_check_pm_mismatch() {
    let temp_dir = std::env::temp_dir().join("agent_hooks_test_mismatch");
    let _ = std::fs::create_dir_all(&temp_dir);

    cleanup_lock_files(&temp_dir);

    std::fs::write(temp_dir.join("pnpm-lock.yaml"), "").unwrap();

    let result = check_package_manager("npm install", &temp_dir);
    assert_eq!(
        result,
        PackageManagerCheckResult::Mismatch {
            command_pm: PackageManager::Npm,
            expected_pm: PackageManager::Pnpm,
        }
    );

    let _ = std::fs::remove_file(temp_dir.join("pnpm-lock.yaml"));
    let _ = std::fs::remove_dir(&temp_dir);
}

#[test]
fn test_check_pm_ambiguous() {
    let temp_dir = std::env::temp_dir().join("agent_hooks_test_ambiguous");
    let _ = std::fs::create_dir_all(&temp_dir);

    cleanup_lock_files(&temp_dir);

    std::fs::write(temp_dir.join("package-lock.json"), "").unwrap();
    std::fs::write(temp_dir.join("pnpm-lock.yaml"), "").unwrap();

    let result = check_package_manager("npm install", &temp_dir);
    match result {
        PackageManagerCheckResult::Ambiguous {
            command_pm,
            detected_pms,
        } => {
            assert_eq!(command_pm, PackageManager::Npm);
            assert!(detected_pms.contains(&PackageManager::Npm));
            assert!(detected_pms.contains(&PackageManager::Pnpm));
        }
        _ => panic!("Expected Ambiguous result, got {result:?}"),
    }

    let _ = std::fs::remove_file(temp_dir.join("package-lock.json"));
    let _ = std::fs::remove_file(temp_dir.join("pnpm-lock.yaml"));
    let _ = std::fs::remove_dir(&temp_dir);
}

#[test]
fn test_check_pm_non_install_command() {
    let temp_dir = std::env::temp_dir().join("agent_hooks_test_non_install");
    let _ = std::fs::create_dir_all(&temp_dir);

    cleanup_lock_files(&temp_dir);

    std::fs::write(temp_dir.join("pnpm-lock.yaml"), "").unwrap();

    // npm run build should not trigger mismatch check
    let result = check_package_manager("npm run build", &temp_dir);
    assert_eq!(result, PackageManagerCheckResult::Ok);

    let _ = std::fs::remove_file(temp_dir.join("pnpm-lock.yaml"));
    let _ = std::fs::remove_dir(&temp_dir);
}
