//! Per-key field extraction plus the pure normalisation helpers.
//!
//! The `normalise_*` helpers are deliberately free of registry access so they
//! can be unit tested on any machine, 技术文档 §13.1.

use winreg::RegKey;

use crate::icons::resolve_icon_path;
use crate::models::{Hive, ProgramInfo, RegistryView};

/// Name prefixes that mark a program as a system component even when the key
/// does not set `SystemComponent`.
pub const SYSTEM_NAME_PREFIXES: &[&str] = &[
    "windows",
    "microsoft",
    "update for",
    "security update",
    "hotfix",
    "driver package",
    "modem",
    "sql",
    "visual c++",
    "asp.net",
    "windows sdk",
    "windows subsystem",
];

pub struct SourceRef {
    pub hive: Hive,
    /// Relative sub path below the hive, no trailing backslash.
    pub sub_path: &'static str,
    pub view: RegistryView,
}

impl SourceRef {
    pub fn root_path(&self) -> String {
        format!("{}\\{}", self.hive.as_str(), self.sub_path)
    }
}

/// Reads a `String` value, trimming and collapsing empties to `None`.
fn read_string(key: &RegKey, name: &str) -> Option<String> {
    let raw: String = key.get_value(name).ok()?;
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

/// Reads a `DWORD` flag, treating anything other than `1` as false.
fn read_flag(key: &RegKey, name: &str) -> bool {
    key.get_value::<u32, _>(name)
        .map(|v| v == 1)
        .unwrap_or(false)
}

/// `yyyyMMdd` → `yyyy-MM-dd`. Other shapes are passed through untouched.
pub fn normalise_install_date(raw: &str) -> String {
    let trimmed = raw.trim();
    let bytes = trimmed.as_bytes();
    if bytes.len() == 8 && bytes.iter().all(|b| b.is_ascii_digit()) {
        format!("{}-{}-{}", &trimmed[0..4], &trimmed[4..6], &trimmed[6..8])
    } else {
        trimmed.to_string()
    }
}

/// Trailing separators and whitespace are noise; empty becomes `None`.
pub fn normalise_path(raw: &str) -> Option<String> {
    let t = raw.trim().trim_end_matches(['\\', '/']).trim();
    if t.is_empty() {
        None
    } else {
        Some(t.to_string())
    }
}

/// The registry stores kilobytes; the UI wants bytes. `<= 0` means "unknown".
pub fn normalise_estimated_size(kb: u32) -> Option<u64> {
    if kb == 0 {
        None
    } else {
        Some(u64::from(kb) * 1024)
    }
}

pub fn looks_like_system_component(name: &str, system_component: bool) -> bool {
    if system_component {
        return true;
    }
    let lower = name.trim().to_lowercase();
    SYSTEM_NAME_PREFIXES.iter().any(|p| lower.starts_with(p))
}

/// Builds a `ProgramInfo` from one uninstall subkey.
///
/// Returns `None` when `DisplayName` is missing or blank (F-104).
pub fn read_program(key: &RegKey, src: &SourceRef, key_name: &str) -> Option<ProgramInfo> {
    let display_name = read_string(key, "DisplayName")?;
    if display_name.trim().is_empty() {
        return None;
    }

    let raw_install_date = read_string(key, "InstallDate");
    let install_date = raw_install_date.as_deref().map(normalise_install_date);
    let estimated_size = key
        .get_value::<u32, _>("EstimatedSize")
        .ok()
        .and_then(normalise_estimated_size);
    let display_icon = read_string(key, "DisplayIcon");
    let icon_path = resolve_icon_path(display_icon.as_deref());
    let system_component = read_flag(key, "SystemComponent");
    let uninstall_string = read_string(key, "UninstallString");
    let quiet_uninstall_string = read_string(key, "QuietUninstallString");

    let id = format!("{}:{}:{}", src.hive.as_str(), src.view.as_flag(), key_name);
    let reg_path = format!("{}\\{}\\{}", src.hive.as_str(), src.sub_path, key_name);

    Some(ProgramInfo {
        id,
        key_name: key_name.to_string(),
        reg_path,
        display_name,
        display_version: read_string(key, "DisplayVersion"),
        publisher: read_string(key, "Publisher"),
        install_date,
        raw_install_date,
        install_location: read_string(key, "InstallLocation").and_then(|v| normalise_path(&v)),
        estimated_size,
        can_uninstall: uninstall_string.is_some() || quiet_uninstall_string.is_some(),
        uninstall_string,
        quiet_uninstall_string,
        display_icon,
        icon_path: icon_path.map(|p| p.to_string_lossy().to_string()),
        system_component,
        windows_installer: read_flag(key, "WindowsInstaller"),
        parent_key_name: read_string(key, "ParentKeyName"),
        no_modify: read_flag(key, "NoModify"),
        no_repair: read_flag(key, "NoRepair"),
        is_system: src.hive == Hive::HkLocalMachine
            && looks_like_system_component(key_name, system_component),
        hive: src.hive,
        view: src.view,
        is_64bit: src.view.is_64bit(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn install_date_yyyymmdd_is_iso() {
        assert_eq!(normalise_install_date("20240115"), "2024-01-15");
        assert_eq!(normalise_install_date("  20240115 "), "2024-01-15");
    }

    #[test]
    fn install_date_other_shapes_pass_through() {
        assert_eq!(normalise_install_date("2024-01-15"), "2024-01-15");
        assert_eq!(normalise_install_date("01/15/2024"), "01/15/2024");
        assert_eq!(normalise_install_date("202413"), "202413");
        assert_eq!(normalise_install_date(""), "");
    }

    #[test]
    fn install_date_rejects_non_digits() {
        // 8 chars but not all digits must not be mangled into a fake date.
        assert_eq!(normalise_install_date("2024AB15"), "2024AB15");
    }

    #[test]
    fn path_trims_trailing_separators() {
        assert_eq!(
            normalise_path(r"C:\Program Files\App\"),
            Some(r"C:\Program Files\App".into())
        );
        assert_eq!(normalise_path("  C:\\App  "), Some(r"C:\App".into()));
        assert_eq!(normalise_path("   "), None);
        assert_eq!(normalise_path(r"\\"), None);
    }

    #[test]
    fn estimated_size_converts_kb_to_bytes() {
        assert_eq!(normalise_estimated_size(0), None);
        assert_eq!(normalise_estimated_size(1), Some(1024));
        assert_eq!(normalise_estimated_size(1234), Some(1_263_616));
    }

    #[test]
    fn system_component_detection() {
        assert!(looks_like_system_component("Microsoft Edge", false));
        assert!(looks_like_system_component("Update for Windows 11", false));
        assert!(looks_like_system_component("anything", true));
        assert!(!looks_like_system_component("Visual Studio Code", false));
        assert!(!looks_like_system_component("7-Zip", false));
    }
}
