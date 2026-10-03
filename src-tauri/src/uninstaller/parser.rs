//! Parses `UninstallString` / `QuietUninstallString` into an executable plus
//! arguments.
//!
//! Design rule (技术文档 §8.2.4): we never *guess* a third-party uninstaller's
//! silent switches. `/qn` is only ever added for MSI, where it is documented
//! Windows Installer behaviour. Guessing `/S` / `/verysilent` risks a "silent
//! failure" — the uninstaller returns success while the program is still
//! installed.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandKind {
    /// `msiexec.exe` — can be waited on, `/qn` is safe to add.
    MsiExec,
    /// `rundll32.exe` — a host with a real exit code.
    Rundll32,
    /// A regular executable; may or may not show UI.
    Runnable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedCommand {
    pub executable: String,
    pub args: Vec<String>,
    pub kind: CommandKind,
    /// Whether the backend should block until the process exits.
    pub can_wait: bool,
    pub original: String,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ParseError {
    #[error("卸载命令为空")]
    Empty,
    #[error("无法识别 MSI 产品代码")]
    NoProductCode,
    #[error("引号未闭合")]
    UnterminatedQuote,
    #[error("找不到可执行文件：{0}")]
    NoExecutable(String),
}

const EXECUTABLE_SUFFIXES: &[&str] = &[".exe", ".com", ".bat", ".cmd"];

/// Splits a command line the way `CommandLineToArgvW` does (simplified but
/// faithful for the shapes that show up in uninstall strings).
pub fn split_args(input: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut has_token = false;
    let mut in_quotes = false;
    let mut chars = input.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '"' => {
                has_token = true;
                in_quotes = !in_quotes;
            }
            '\\' => {
                // Count the run of backslashes to decide whether the next char
                // is an escaped quote.
                let mut slashes = 1;
                while chars.peek() == Some(&'\\') {
                    chars.next();
                    slashes += 1;
                }
                if chars.peek() == Some(&'"') {
                    chars.next();
                    // Every pair of backslashes becomes one literal backslash.
                    for _ in 0..slashes / 2 {
                        current.push('\\');
                    }
                    if slashes % 2 == 1 {
                        // `\"`  — the quote is escaped, so it is a literal.
                        current.push('"');
                    } else {
                        // `\\"` — the quote is a delimiter.
                        in_quotes = !in_quotes;
                    }
                    has_token = true;
                } else {
                    for _ in 0..slashes {
                        current.push('\\');
                    }
                }
            }
            c if c.is_whitespace() && !in_quotes => {
                if has_token {
                    args.push(std::mem::take(&mut current));
                    has_token = false;
                }
            }
            c => {
                has_token = true;
                current.push(c);
            }
        }
    }
    if has_token {
        args.push(current);
    }
    args
}

fn executable_name(path: &str) -> String {
    let file = path
        .rsplit(['\\', '/'])
        .next()
        .unwrap_or(path)
        .trim_matches('"');
    file.to_ascii_lowercase()
}

fn classify(executable: &str) -> CommandKind {
    match executable_name(executable).as_str() {
        "msiexec.exe" | "msiexec" => CommandKind::MsiExec,
        "rundll32.exe" | "rundll32" => CommandKind::Rundll32,
        _ => CommandKind::Runnable,
    }
}

/// MSI is safe to wait on; `rundll32` reports the DLL's exit code. Anything
/// else is a GUI uninstaller we cannot observe, so we report "started".
fn can_wait_for(kind: CommandKind) -> bool {
    matches!(kind, CommandKind::MsiExec | CommandKind::Rundll32)
}

/// Finds the first `{...}` product code, falling back to a bare GUID.
fn find_product_code(cmd: &str) -> Option<String> {
    if let Some(start) = cmd.find('{') {
        if let Some(rel_end) = cmd[start + 1..].find('}') {
            return Some(cmd[start..start + rel_end + 2].to_string());
        }
    }
    // Bare `8.4.4.4-...` GUIDs also appear in the wild.
    let mut best: Option<String> = None;
    for token in cmd.split_whitespace() {
        let t = token.trim_matches('"').trim_start_matches('/');
        if t.len() == 36
            && t.chars().filter(|c| *c == '-').count() == 4
            && t.chars().all(|c| c.is_ascii_hexdigit() || c == '-')
        {
            best = Some(t.to_string());
            break;
        }
    }
    best
}

fn is_msi_command(cmd: &str) -> bool {
    // Look at the *first* token only, so a path such as
    // `C:\tools\msiexec-helper\un.exe` is not mistaken for MSI.
    let head = cmd.trim().trim_start_matches('"');
    let head = head.split_whitespace().next().unwrap_or(head);
    let head = head.trim_matches('"');
    let name = executable_name(head);
    name == "msiexec.exe" || name == "msiexec"
}

/// Parses an MSI uninstall command, if that is what this is.
fn parse_msi(cmd: &str) -> Result<Option<ParsedCommand>, ParseError> {
    if !is_msi_command(cmd) {
        return Ok(None);
    }
    let guid = find_product_code(cmd).ok_or(ParseError::NoProductCode)?;
    let lower = cmd.to_ascii_lowercase();

    let mut args = vec!["/X".to_string(), guid];
    // Preserve an existing silent intent, but never invent one.
    if lower.contains("/qn") || lower.contains("quiet") || lower.contains("passive") {
        args.push("/qn".to_string());
    }
    args.push("/norestart".to_string());

    Ok(Some(ParsedCommand {
        executable: "msiexec.exe".to_string(),
        args,
        kind: CommandKind::MsiExec,
        can_wait: true,
        original: cmd.to_string(),
    }))
}

fn split_quoted_head(cmd: &str) -> Result<(String, &str), ParseError> {
    let rest = &cmd[1..];
    let end = rest.find('"').ok_or(ParseError::UnterminatedQuote)?;
    let exe = &rest[..end];
    if exe.is_empty() {
        return Err(ParseError::NoExecutable(cmd.to_string()));
    }
    Ok((exe.to_string(), rest[end + 1..].trim()))
}

fn split_first_ws(cmd: &str) -> (String, &str) {
    match cmd.find(char::is_whitespace) {
        Some(idx) => (cmd[..idx].to_string(), cmd[idx..].trim_start()),
        None => (cmd.to_string(), ""),
    }
}

/// Handles malformed unquoted paths such as `C:\Program Files\App\un.exe /S`.
///
/// Walks every whitespace position looking for a head that ends in a known
/// executable suffix; the split at the first space is kept as a fallback.
fn resolve_bare_path(cmd: &str) -> Option<(String, Vec<String>)> {
    let mut fallback: Option<(String, Vec<String>)> = None;
    for (idx, _) in cmd.match_indices(char::is_whitespace) {
        let head = cmd[..idx].trim();
        if head.is_empty() {
            continue;
        }
        let tail = cmd[idx..].trim();
        let args = if tail.is_empty() {
            Vec::new()
        } else {
            split_args(tail)
        };
        let lower = executable_name(head);
        if EXECUTABLE_SUFFIXES.iter().any(|s| lower.ends_with(s)) {
            return Some((head.to_string(), args));
        }
        fallback.get_or_insert_with(|| (head.to_string(), args));
    }
    fallback
}

/// Entry point: turns a registry uninstall string into a runnable command.
pub fn parse_uninstall_string(cmd: &str) -> Result<ParsedCommand, ParseError> {
    let trimmed = cmd.trim();
    if trimmed.is_empty() {
        return Err(ParseError::Empty);
    }

    if let Some(parsed) = parse_msi(trimmed)? {
        return Ok(parsed);
    }

    if trimmed.starts_with('"') {
        let (exe, rest) = split_quoted_head(trimmed)?;
        let kind = classify(&exe);
        return Ok(ParsedCommand {
            can_wait: can_wait_for(kind),
            executable: exe,
            args: if rest.is_empty() {
                Vec::new()
            } else {
                split_args(rest)
            },
            kind,
            original: trimmed.to_string(),
        });
    }

    let (exe, rest) = split_first_ws(trimmed);
    let kind = classify(&exe);
    if kind != CommandKind::Runnable {
        // e.g. `msiexec.exe /x{...}` handled above; anything else keeps its args.
        return Ok(ParsedCommand {
            can_wait: can_wait_for(kind),
            executable: exe,
            args: if rest.is_empty() {
                Vec::new()
            } else {
                split_args(rest)
            },
            kind,
            original: trimmed.to_string(),
        });
    }

    // `Runnable` with an unquoted path: retry with the executable-suffix rule.
    if let Some((exe, args)) = resolve_bare_path(trimmed) {
        let kind = classify(&exe);
        return Ok(ParsedCommand {
            can_wait: can_wait_for(kind),
            executable: exe,
            args,
            kind,
            original: trimmed.to_string(),
        });
    }

    Ok(ParsedCommand {
        can_wait: false,
        executable: exe,
        args: Vec::new(),
        kind,
        original: trimmed.to_string(),
    })
}

/// Applies the quiet-uninstall policy (技术文档 §8.2.4).
///
/// Returns the command to run plus whether we had to fall back to interactive.
pub fn choose_command(
    uninstall_string: Option<&str>,
    quiet_string: Option<&str>,
    want_quiet: bool,
) -> Result<(ParsedCommand, bool), ParseError> {
    if want_quiet {
        if let Some(q) = quiet_string.map(str::trim).filter(|s| !s.is_empty()) {
            return Ok((parse_uninstall_string(q)?, false));
        }
    }
    let base = uninstall_string
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or(ParseError::NoExecutable("未提供 UninstallString".into()))?;
    let parsed = parse_uninstall_string(base)?;
    if !want_quiet {
        return Ok((parsed, false));
    }
    // Quiet was requested but there is no QuietUninstallString. `/qn` is only
    // safe for MSI, which `parse_uninstall_string` already normalised.
    let quiet_fallback = parsed.kind != CommandKind::MsiExec;
    Ok((parsed, quiet_fallback))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(s: &str) -> ParsedCommand {
        parse_uninstall_string(s).unwrap_or_else(|e| panic!("parse failed for {s:?}: {e}"))
    }

    // ---- T-101: the documented case table --------------------------------

    #[test]
    fn t101_empty_inputs() {
        assert_eq!(parse_uninstall_string(""), Err(ParseError::Empty));
        assert_eq!(parse_uninstall_string("   "), Err(ParseError::Empty));
    }

    #[test]
    fn t101_quoted_path_no_args() {
        let p = parse(r#""C:\Program Files\A\un.exe""#);
        assert_eq!(p.executable, r"C:\Program Files\A\un.exe");
        assert!(p.args.is_empty());
        assert!(!p.can_wait);
        assert_eq!(p.kind, CommandKind::Runnable);
    }

    #[test]
    fn t101_quoted_path_with_switch() {
        let p = parse(r#""C:\Program Files\A\un.exe" /S"#);
        assert_eq!(p.executable, r"C:\Program Files\A\un.exe");
        assert_eq!(p.args, vec!["/S"]);
    }

    #[test]
    fn t101_msi_install_switch_becomes_uninstall() {
        let p = parse(r"MsiExec.exe /I{11111111-2222-3333-4444-555555555555}");
        assert_eq!(p.executable, "msiexec.exe");
        assert_eq!(
            p.args,
            vec!["/X", "{11111111-2222-3333-4444-555555555555}", "/norestart"]
        );
        assert!(p.can_wait);
        assert_eq!(p.kind, CommandKind::MsiExec);
    }

    #[test]
    fn t101_msi_uninstall_switch_keeps_quiet() {
        let p = parse(r"MsiExec.exe /X{11111111-2222-3333-4444-555555555555} /qn");
        assert_eq!(
            p.args,
            vec![
                "/X",
                "{11111111-2222-3333-4444-555555555555}",
                "/qn",
                "/norestart"
            ]
        );
    }

    #[test]
    fn t101_quoted_msi_with_env_path() {
        let p =
            parse(r#""C:\Windows\system32\msiexec.exe" /X{11111111-2222-3333-4444-555555555555}"#);
        assert_eq!(p.executable, "msiexec.exe");
        assert!(p.can_wait);
    }

    #[test]
    fn t101_two_quoted_arguments() {
        let p = parse(r#""C:\a b\un.exe" "C:\x y\z.dat""#);
        assert_eq!(p.executable, r"C:\a b\un.exe");
        assert_eq!(p.args, vec![r"C:\x y\z.dat"]);
    }

    #[test]
    fn t101_short_path_with_switches() {
        let p = parse(r"C:\PROGRA~1\A\uninst.exe /quiet /norestart");
        assert_eq!(p.executable, r"C:\PROGRA~1\A\uninst.exe");
        assert_eq!(p.args, vec!["/quiet", "/norestart"]);
    }

    #[test]
    fn t101_rundll32_is_waitable() {
        let p = parse(r#"rundll32.exe "C:\P\un.dll,Uninstall""#);
        assert_eq!(p.executable, "rundll32.exe");
        assert_eq!(p.args, vec![r"C:\P\un.dll,Uninstall"]);
        assert!(p.can_wait);
        assert_eq!(p.kind, CommandKind::Rundll32);
    }

    #[test]
    fn t101_msi_without_guid_errors() {
        assert_eq!(
            parse_uninstall_string(r"msiexec.exe /I"),
            Err(ParseError::NoProductCode)
        );
    }

    #[test]
    fn t101_escaped_quote_argument() {
        let p = parse(r#""C:\a\un.exe" "say \"hi\"""#);
        assert_eq!(p.args, vec![r#"say "hi""#]);
    }

    #[test]
    fn t101_chinese_path() {
        let p = parse(r#""D:\软件\卸载.exe" /S"#);
        assert_eq!(p.executable, r"D:\软件\卸载.exe");
        assert_eq!(p.args, vec!["/S"]);
    }

    // ---- additional edge cases -------------------------------------------

    #[test]
    fn unterminated_quote_is_reported() {
        assert_eq!(
            parse_uninstall_string(r#""C:\a\un.exe"#),
            Err(ParseError::UnterminatedQuote)
        );
    }

    #[test]
    fn unquoted_path_with_spaces_is_recovered() {
        // Malformed but common: no quotes around a path that contains spaces.
        let p = parse(r"C:\Program Files\App\un.exe /S");
        assert_eq!(p.executable, r"C:\Program Files\App\un.exe");
        assert_eq!(p.args, vec!["/S"]);
    }

    #[test]
    fn path_containing_msiexec_is_not_treated_as_msi() {
        let p = parse(r#""C:\tools\msiexec-helper\un.exe" /S"#);
        assert_eq!(p.kind, CommandKind::Runnable);
        assert_eq!(p.executable, r"C:\tools\msiexec-helper\un.exe");
    }

    #[test]
    fn powershell_command_with_quoted_script() {
        let p = parse(r#"powershell -NoProfile -ExecutionPolicy Bypass -File "C:\p.ps1""#);
        assert_eq!(p.executable, "powershell");
        assert_eq!(
            p.args,
            vec![
                "-NoProfile",
                "-ExecutionPolicy",
                "Bypass",
                "-File",
                r"C:\p.ps1"
            ]
        );
    }

    #[test]
    fn very_long_command_is_accepted() {
        let long = format!(r#""C:\a\un.exe"{}"#, " /x".repeat(600));
        let p = parse(&long);
        assert_eq!(p.args.len(), 600);
    }

    #[test]
    fn bare_guid_msi_without_braces() {
        let p = parse(r"MsiExec.exe /I 11111111-2222-3333-4444-555555555555");
        assert_eq!(p.args[0], "/X");
        assert_eq!(p.args[1], "11111111-2222-3333-4444-555555555555");
    }

    #[test]
    fn original_string_is_preserved() {
        let raw = r#""C:\a\un.exe" /S"#;
        assert_eq!(parse(raw).original, raw);
    }

    // ---- split_args ------------------------------------------------------

    #[test]
    fn split_args_edge_cases() {
        assert!(split_args("").is_empty());
        assert!(split_args("   ").is_empty());
        assert_eq!(split_args(r#""""#), vec![""]);
        assert_eq!(split_args(r#"a "b c" d"#), vec!["a", "b c", "d"]);
        assert_eq!(split_args(r#"a  b"#), vec!["a", "b"]);
        assert_eq!(split_args(r#""a b""#), vec!["a b"]);
    }

    // ---- quiet policy ----------------------------------------------------

    #[test]
    fn quiet_prefers_quiet_string() {
        let (p, fallback) =
            choose_command(Some(r#""C:\a\un.exe""#), Some(r#""C:\a\un.exe" /S"#), true).unwrap();
        assert_eq!(p.args, vec!["/S"]);
        assert!(!fallback);
    }

    #[test]
    fn quiet_without_quiet_string_falls_back_for_exe() {
        let (p, fallback) = choose_command(Some(r#""C:\a\un.exe""#), None, true).unwrap();
        assert!(p.args.is_empty(), "no /S may be invented");
        assert!(fallback);
    }

    #[test]
    fn quiet_without_quiet_string_is_clean_for_msi() {
        let (p, fallback) = choose_command(
            Some(r"MsiExec.exe /I{11111111-2222-3333-4444-555555555555}"),
            None,
            true,
        )
        .unwrap();
        assert_eq!(p.args[0], "/X");
        assert!(
            !p.args.contains(&"/qn".to_string()),
            "no silent intent to preserve"
        );
        assert!(!fallback);
    }

    #[test]
    fn no_quiet_string_at_all_errors() {
        assert!(choose_command(None, None, false).is_err());
        assert!(choose_command(Some("   "), None, false).is_err());
    }

    #[test]
    fn interactive_mode_uses_uninstall_string() {
        let (p, fallback) = choose_command(
            Some(r#""C:\a\un.exe" /S"#),
            Some(r#""C:\a\un.exe" /S /NOCONFIRM"#),
            false,
        )
        .unwrap();
        assert_eq!(p.args, vec!["/S"]);
        assert!(!fallback);
    }
}
