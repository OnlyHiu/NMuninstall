//! Integration coverage for the public surface of the uninstall parser.
//!
//! `parser.rs` has thorough unit tests; this file pins down that the items are
//! actually `pub` and usable from outside the crate, so a refactor cannot
//! quietly break the boundary.

use nmuninstall_lib::uninstaller::parser::{
    choose_command, parse_uninstall_string, split_args, CommandKind, ParseError,
};

#[test]
fn parses_a_quoted_uninstaller() {
    let parsed = parse_uninstall_string(r#""C:\Program Files\Vendor App\unins000.exe""#)
        .expect("should parse");
    assert_eq!(
        parsed.executable,
        r"C:\Program Files\Vendor App\unins000.exe"
    );
    assert!(parsed.args.is_empty());
    assert_eq!(parsed.kind, CommandKind::Runnable);
    assert!(!parsed.can_wait);
}

#[test]
fn parses_msi_and_waits_for_it() {
    let parsed = parse_uninstall_string(r"MsiExec.exe /I{0A1B2C3D-4E5F-6071-8293-A4B5C6D7E8F9}")
        .expect("should parse");
    assert_eq!(parsed.kind, CommandKind::MsiExec);
    assert!(parsed.can_wait);
    assert_eq!(parsed.args[0], "/X", "install switch must become uninstall");
}

#[test]
fn reports_missing_product_codes() {
    assert_eq!(
        parse_uninstall_string("msiexec.exe /I"),
        Err(ParseError::NoProductCode)
    );
}

#[test]
fn reports_empty_input() {
    assert_eq!(parse_uninstall_string("   "), Err(ParseError::Empty));
}

#[test]
fn handles_chinese_paths() {
    let parsed = parse_uninstall_string(r#""D:\软件\卸载程序.exe" /S"#).expect("should parse");
    assert_eq!(parsed.executable, r"D:\软件\卸载程序.exe");
    assert_eq!(parsed.args, vec!["/S"]);
}

#[test]
fn split_args_is_public_and_correct() {
    assert_eq!(split_args(r#"a "b c" d"#), vec!["a", "b c", "d"]);
    assert!(split_args("  ").is_empty());
}

#[test]
fn quiet_policy_prefers_the_quiet_string() {
    let (parsed, fallback) =
        choose_command(Some(r#""C:\a\un.exe""#), Some(r#""C:\a\un.exe" /S"#), true)
            .expect("should parse");
    assert_eq!(parsed.args, vec!["/S"]);
    assert!(!fallback, "an explicit quiet string is never a fallback");
}

#[test]
fn quiet_policy_refuses_to_invent_switches() {
    let (parsed, fallback) =
        choose_command(Some(r#""C:\a\un.exe""#), None, true).expect("should parse");
    assert!(parsed.args.is_empty());
    assert!(fallback, "a non-MSI program must be reported as a fallback");
}
