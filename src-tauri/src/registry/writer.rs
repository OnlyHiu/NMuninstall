//! The *only* place in the codebase that deletes a registry key.
//!
//! Everything funnels through [`delete_subkey_with_audit`] so that every write
//! is logged and every path is validated first (技术文档 §8.5).

use tracing::{info, warn};
use winreg::enums::KEY_ALL_ACCESS;

use crate::error::{AppError, AppResult};
use crate::models::Hive;

/// Splits `HKLM\SOFTWARE\Foo\Bar` into `("HKLM", "SOFTWARE\\Foo\\Bar")`.
pub fn split_hive(path: &str) -> Option<(&str, &str)> {
    let path = path.trim();
    let (head, tail) = path.split_once('\\')?;
    let tail = tail.trim_matches('\\');
    if tail.is_empty() {
        None
    } else {
        Some((head, tail))
    }
}

/// Rejects anything that is not a single leaf under one of the three
/// `Uninstall` roots. Guards against a frontend-supplied path reaching a
/// sensitive key even if the caller is buggy.
pub fn is_uninstall_subkey(path: &str) -> bool {
    let Some((head, tail)) = split_hive(path) else {
        return false;
    };
    if Hive::parse(head).is_none() {
        return false;
    }

    let leaves: Vec<String> = tail
        .replace('/', "\\")
        .split('\\')
        .filter(|s| !s.is_empty())
        .map(|s| s.to_lowercase())
        .collect();

    // No traversal segments, ever.
    if leaves.iter().any(|l| l == "." || l == "..") {
        return false;
    }

    // Find the last `Uninstall` component; require real parents above it and at
    // least one key below it.
    let Some(idx) = leaves.iter().rposition(|l| l == "uninstall") else {
        return false;
    };
    // At least `SOFTWARE\<vendor>\<product>\Uninstall` above the leaf.
    idx >= 3 && idx + 1 < leaves.len()
}

fn parent_and_leaf(sub: &str) -> (&str, &str) {
    let trimmed = sub.trim_end_matches('\\');
    match trimmed.rfind('\\') {
        Some(idx) => (&trimmed[..idx], &trimmed[idx + 1..]),
        None => ("", trimmed),
    }
}

/// Deletes `path` from the registry after validating it.
///
/// `actor` is recorded in the log so the audit trail identifies the caller
/// (`residue::clean`, `settings`, ...).
pub fn delete_subkey_with_audit(path: &str, actor: &str) -> AppResult<()> {
    if !is_uninstall_subkey(path) {
        return Err(AppError::PathNotAllowed(path.to_string()));
    }
    let (head, sub) = split_hive(path).ok_or_else(|| AppError::PathNotAllowed(path.to_string()))?;
    let hive = Hive::parse(head).ok_or_else(|| AppError::PathNotAllowed(path.to_string()))?;
    let (parent, leaf) = parent_and_leaf(sub);
    if parent.is_empty() || leaf.is_empty() {
        return Err(AppError::PathNotAllowed(path.to_string()));
    }

    info!(path, actor, "registry delete requested");

    hive.predef()
        .open_subkey_with_flags(parent, KEY_ALL_ACCESS)
        .and_then(|k| k.delete_subkey(leaf))
        .map_err(|e| {
            warn!(path, actor, error = %e, "registry delete failed");
            AppError::CleanFailed(format!("{path}: {e}"))
        })
}

/// `true` when the key still exists. Used by residue detection.
pub fn key_exists(path: &str) -> bool {
    let Some((head, sub)) = split_hive(path) else {
        return false;
    };
    let Some(hive) = Hive::parse(head) else {
        return false;
    };
    let flags =
        winreg::enums::KEY_READ | winreg::enums::KEY_WOW64_64KEY | winreg::enums::KEY_WOW64_32KEY;
    hive.predef().open_subkey_with_flags(sub, flags).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_hive_works() {
        assert_eq!(
            split_hive(r"HKLM\SOFTWARE\App\Uninstall\{guid}"),
            Some((r"HKLM", r"SOFTWARE\App\Uninstall\{guid}"))
        );
        assert_eq!(split_hive("HKLM"), None);
        assert_eq!(split_hive(r"HKLM\"), None);
    }

    #[test]
    fn accepts_real_uninstall_subkeys() {
        assert!(is_uninstall_subkey(
            r"HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\{ABCD-1234}"
        ));
        assert!(is_uninstall_subkey(
            r"HKCU\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\Google Chrome"
        ));
        assert!(is_uninstall_subkey(
            r"HKLM\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\Foo"
        ));
    }

    #[test]
    fn rejects_dangerous_or_malformed_paths() {
        assert!(!is_uninstall_subkey(
            r"HKLM\SYSTEM\CurrentControlSet\Services"
        ));
        assert!(!is_uninstall_subkey(
            r"HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Run"
        ));
        assert!(!is_uninstall_subkey(r"HKCR\*\.txt"));
        assert!(!is_uninstall_subkey("garbage"));
        assert!(!is_uninstall_subkey(
            r"HKLM\SOFTWARE\CurrentVersion\Uninstall"
        ));
        assert!(!is_uninstall_subkey(
            r"HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\..\..\Run"
        ));
    }

    #[test]
    fn parent_and_leaf_splits_once() {
        assert_eq!(
            parent_and_leaf(r"SOFTWARE\App\Uninstall\{guid}"),
            (r"SOFTWARE\App\Uninstall", "{guid}")
        );
        assert_eq!(parent_and_leaf("Uninstall"), ("", "Uninstall"));
    }
}
