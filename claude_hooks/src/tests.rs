//! Unit tests for claude_hooks

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
    let result = deny_rust_allow(&ToolName::Edit, &input, &default_options());
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
    let result = deny_rust_allow(&ToolName::Write, &input, &default_options());
    assert!(result.is_some());
}

#[test]
fn test_deny_rust_allow_detects_expect_without_flag() {
    let input = make_tool_input("src/main.rs", "#[expect(dead_code)]");
    let result = deny_rust_allow(&ToolName::Edit, &input, &default_options());
    assert!(result.is_some()); // Should deny #[expect] too
}

#[test]
fn test_deny_rust_allow_allows_expect_with_flag() {
    let input = make_tool_input("src/main.rs", "#[expect(dead_code)]");
    let options = DenyRustAllowOptions {
        expect: true,
        additional_context: None,
    };
    let result = deny_rust_allow(&ToolName::Edit, &input, &options);
    assert!(result.is_none()); // Should allow #[expect]
}

#[test]
fn test_deny_rust_allow_denies_allow_with_expect_flag() {
    let input = make_tool_input("src/main.rs", "#[allow(dead_code)]");
    let options = DenyRustAllowOptions {
        expect: true,
        additional_context: None,
    };
    let result = deny_rust_allow(&ToolName::Edit, &input, &options);
    assert!(result.is_some()); // Should still deny #[allow]
}

#[test]
fn test_deny_rust_allow_ignores_non_rust_files() {
    let input = make_tool_input("README.md", "#[allow(dead_code)]");
    let result = deny_rust_allow(&ToolName::Edit, &input, &default_options());
    assert!(result.is_none());
}

#[test]
fn test_deny_rust_allow_ignores_comments() {
    let input = make_tool_input("src/main.rs", "// #[allow(dead_code)]\nfn foo() {}");
    let result = deny_rust_allow(&ToolName::Edit, &input, &default_options());
    assert!(result.is_none());
}

#[test]
fn test_deny_rust_allow_ignores_string_literals() {
    // Content: let s = "#[allow(dead_code)]";
    let input = make_tool_input("src/main.rs", "let s = \"#[allow(dead_code)]\";");
    let result = deny_rust_allow(&ToolName::Edit, &input, &default_options());
    assert!(result.is_none());
}

#[test]
fn test_deny_rust_allow_ignores_wrong_tool() {
    let input = make_tool_input("src/main.rs", "#[allow(dead_code)]");
    let result = deny_rust_allow(&ToolName::Bash, &input, &default_options());
    assert!(result.is_none());
}

#[test]
fn test_deny_rust_allow_additional_context() {
    let input = make_tool_input("src/main.rs", "#[allow(dead_code)]");
    let options = DenyRustAllowOptions {
        expect: false,
        additional_context: Some("See guidelines".to_string()),
    };
    let result = deny_rust_allow(&ToolName::Edit, &input, &options);
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
    let result = deny_rust_allow(&ToolName::Edit, &input, &default_options());
    assert!(result.is_some()); // Should match .RS as well
}

#[test]
fn test_deny_rust_allow_allows_normal_code() {
    let input = make_tool_input("src/main.rs", "fn foo() { println!(\"hello\"); }");
    let result = deny_rust_allow(&ToolName::Edit, &input, &default_options());
    assert!(result.is_none());
}
