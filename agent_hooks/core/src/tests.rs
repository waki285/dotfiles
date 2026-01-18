//! Unit tests for agent_hooks core

use super::*;

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
