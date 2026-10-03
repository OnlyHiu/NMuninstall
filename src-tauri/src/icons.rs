//! Resolves the `DisplayIcon` registry value into a usable absolute path.
//!
//! `DisplayIcon` has three shapes in the wild:
//!   `C:\path\app.exe,0`      — explicit resource index
//!   `C:\path\app.exe`        — bare path
//!   `%SystemRoot%\system32\a.dll,-101`
//!
//! We only return the path when the file actually exists, so the frontend can
//! hand it straight to the asset protocol without a broken-image flash.

use std::path::PathBuf;

use tracing::trace;

/// Longest expansion we accept, to stay inside the legacy `MAX_PATH` budget.
const MAX_EXPANDED_LEN: usize = 260;

/// Splits a trailing `,<integer>` index suffix, but only when the suffix really
/// is an integer (`C:\a,b.exe` must survive intact).
pub fn strip_icon_index(raw: &str) -> &str {
    match raw.rsplit_once(',') {
        Some((head, tail)) if tail.trim().parse::<i32>().is_ok() => head,
        _ => raw,
    }
}

/// Expands `%VAR%` references present in the string.
///
/// Fails closed (returns `None`) when a variable cannot be resolved or when the
/// expansion would exceed the legacy `MAX_PATH` budget.
pub fn expand_env_vars(input: &str) -> Option<String> {
    let mut out = String::with_capacity(input.len());
    let mut rest = input;
    while let Some(start) = rest.find('%') {
        out.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        let Some(end) = after.find('%') else {
            // Unterminated `%` — keep the remainder verbatim.
            out.push('%');
            out.push_str(after);
            return finish(out);
        };
        let name = &after[..end];
        if name.is_empty() {
            out.push('%');
            rest = &after[end + 1..];
            continue;
        }
        // `%SystemRoot%` and friends are present in the process environment on
        // Windows; an unresolved name aborts expansion (fail closed).
        out.push_str(&std::env::var(name).ok()?);
        rest = &after[end + 1..];
    }
    out.push_str(rest);
    finish(out)
}

fn finish(out: String) -> Option<String> {
    if out.len() > MAX_EXPANDED_LEN {
        None
    } else {
        Some(out)
    }
}

/// Full resolution: strip index, expand env vars, verify existence.
pub fn resolve_icon_path(display_icon: Option<&str>) -> Option<PathBuf> {
    let raw = display_icon?.trim();
    if raw.is_empty() {
        return None;
    }
    let without_index = strip_icon_index(raw).trim().trim_matches('"').trim();
    if without_index.is_empty() {
        return None;
    }
    let expanded = if without_index.contains('%') {
        expand_env_vars(without_index)?
    } else {
        without_index.to_string()
    };
    let path = PathBuf::from(&expanded);
    if !path.is_file() {
        trace!(icon = %expanded, "DisplayIcon target missing, hiding icon");
        return None;
    }
    Some(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_only_integer_suffixes() {
        assert_eq!(strip_icon_index(r"C:\a\app.exe,0"), r"C:\a\app.exe");
        assert_eq!(strip_icon_index(r"C:\a\app.exe,-101"), r"C:\a\app.exe");
        assert_eq!(strip_icon_index(r"C:\a\app.exe"), r"C:\a\app.exe");
        // Not an index, so it belongs to the file name.
        assert_eq!(strip_icon_index(r"C:\a\b.exe"), r"C:\a\b.exe");
    }

    #[test]
    fn env_expansion_resolves_system_root() {
        let out = expand_env_vars(r"%SystemRoot%\system32\kernel32.dll").expect("expansion");
        assert!(out.to_lowercase().ends_with(r"system32\kernel32.dll"));
        assert!(!out.contains('%'));
    }

    #[test]
    fn env_expansion_leaves_plain_strings_alone() {
        assert_eq!(
            expand_env_vars(r"C:\Program Files\App\app.exe").unwrap(),
            r"C:\Program Files\App\app.exe"
        );
    }

    #[test]
    fn env_expansion_rejects_unknown_variables() {
        assert!(expand_env_vars(r"%NM_DEFINITELY_NOT_SET_VAR_12345%\x").is_none());
    }

    #[test]
    fn resolves_existing_file_only() {
        // A file that certainly exists on Windows.
        let got = resolve_icon_path(Some(r"C:\Windows\System32\kernel32.dll,0"));
        assert!(got.is_some(), "kernel32.dll should resolve");
        assert!(resolve_icon_path(Some(r"C:\definitely\not\here.exe")).is_none());
        assert!(resolve_icon_path(None).is_none());
        assert!(resolve_icon_path(Some("   ")).is_none());
    }
}
