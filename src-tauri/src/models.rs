//! Data structures shared between the Rust backend and the React frontend.
//!
//! Every struct serialises to camelCase so TypeScript can consume it directly.

use serde::{Deserialize, Serialize};

/// Renamed explicitly: the wire format is the short hive name the TypeScript
/// `Hive` union uses (`'HKLM' | 'HKCU'`), not the Rust variant name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Hive {
    #[serde(rename = "HKLM")]
    HkLocalMachine,
    #[serde(rename = "HKCU")]
    HkCurrentUser,
}

impl Hive {
    pub fn as_str(&self) -> &'static str {
        match self {
            Hive::HkLocalMachine => "HKLM",
            Hive::HkCurrentUser => "HKCU",
        }
    }

    #[cfg(windows)]
    pub fn as_hkey(&self) -> winreg::HKEY {
        match self {
            Hive::HkLocalMachine => winreg::enums::HKEY_LOCAL_MACHINE,
            Hive::HkCurrentUser => winreg::enums::HKEY_CURRENT_USER,
        }
    }

    /// `winreg` 0.55 exposes its API on `RegKey`, so predefined hives are
    /// wrapped before use.
    #[cfg(windows)]
    pub fn predef(&self) -> winreg::RegKey {
        winreg::RegKey::predef(self.as_hkey())
    }

    /// Parses `HKLM` / `HKCU` (case-insensitive). Returns `None` for anything else.
    pub fn parse(s: &str) -> Option<Hive> {
        match s.trim().to_ascii_uppercase().as_str() {
            "HKLM" | "HKEY_LOCAL_MACHINE" => Some(Hive::HkLocalMachine),
            "HKCU" | "HKEY_CURRENT_USER" => Some(Hive::HkCurrentUser),
            _ => None,
        }
    }
}

/// Renamed explicitly to match the TypeScript `RegistryView` union
/// (`'64' | '32'`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RegistryView {
    #[serde(rename = "64")]
    Bit64,
    #[serde(rename = "32")]
    Bit32,
}

impl RegistryView {
    pub fn as_flag(&self) -> &'static str {
        match self {
            RegistryView::Bit64 => "64",
            RegistryView::Bit32 => "32",
        }
    }

    pub fn is_64bit(&self) -> bool {
        matches!(self, RegistryView::Bit64)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgramInfo {
    /// `{hive}:{view}:{subkey}` — stable within one scan, see 技术文档 §6.1.
    pub id: String,
    pub key_name: String,
    /// Full registry path including the hive, used for residue detection.
    pub reg_path: String,
    pub display_name: String,
    pub display_version: Option<String>,
    pub publisher: Option<String>,
    /// Normalised to `yyyy-MM-dd` when the raw value is 8 digits.
    pub install_date: Option<String>,
    pub raw_install_date: Option<String>,
    pub install_location: Option<String>,
    /// Bytes (the registry stores kilobytes).
    pub estimated_size: Option<u64>,
    pub uninstall_string: Option<String>,
    pub quiet_uninstall_string: Option<String>,
    pub display_icon: Option<String>,
    /// Resolved absolute path, only present when the file actually exists.
    pub icon_path: Option<String>,
    pub system_component: bool,
    pub windows_installer: bool,
    pub parent_key_name: Option<String>,
    pub no_modify: bool,
    pub no_repair: bool,
    pub is_system: bool,
    pub hive: Hive,
    pub view: RegistryView,
    /// Explicit rename: serde's implicit camelCase turns `is_64bit` into
    /// `is64bit`, but the TypeScript contract says `is64Bit`. A mismatched key
    /// is silently `undefined` on the JS side, so it is pinned here instead of
    /// inferred — see `serialised_keys_match_the_typescript_contract`.
    #[serde(rename = "is64Bit")]
    pub is_64bit: bool,
    pub can_uninstall: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanStats {
    pub total: usize,
    pub skipped_no_name: usize,
    pub skipped_inaccessible: usize,
    pub deduplicated: usize,
    pub scanned_keys: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgramsResponse {
    pub programs: Vec<ProgramInfo>,
    pub elapsed_ms: u64,
    pub stats: ScanStats,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UninstallStatus {
    Completed,
    Started,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UninstallResult {
    pub status: UninstallStatus,
    pub message: String,
    /// The executable actually launched, after parsing.
    pub executable: String,
    pub args: Vec<String>,
    pub exit_code: Option<i32>,
    pub waited: bool,
    pub duration_ms: u64,
    /// True when the quiet flag was requested but had to fall back to interactive.
    pub quiet_fallback: bool,
    pub residue: Option<ResidueReport>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResidueItem {
    pub id: String,
    /// `"registry"` or `"directory"`.
    pub kind: String,
    pub path: String,
    pub size_bytes: Option<u64>,
    pub file_count: Option<u64>,
    pub removable: bool,
    pub reason_if_locked: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResidueReport {
    pub registry_keys: Vec<ResidueItem>,
    pub install_dirs: Vec<ResidueItem>,
    pub cleanable_count: usize,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanResult {
    pub removed: Vec<String>,
    pub failed: Vec<CleanFailure>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanFailure {
    pub path: String,
    pub error: String,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    #[default]
    System,
    Light,
    Dark,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Language {
    #[default]
    #[serde(rename = "zh-CN")]
    ZhCn,
    #[serde(rename = "en-US")]
    EnUs,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ListDensity {
    #[default]
    Comfortable,
    Compact,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AppSettings {
    pub show_system_components: bool,
    /// Explicit rename for the same reason as `ProgramInfo::is_64bit`:
    /// implicit camelCase yields `show32bitPrograms`, not the `show32BitPrograms`
    /// the TypeScript contract uses. Getting this wrong made the 32-bit toggle
    /// read back as `undefined`, which JSON then dropped, so `list_programs`
    /// rejected the call and the program list never loaded.
    #[serde(rename = "show32BitPrograms")]
    pub show_32bit_programs: bool,
    pub theme: Theme,
    pub confirm_before_uninstall: bool,
    pub check_residue_after_uninstall: bool,
    pub language: Language,
    pub list_density: ListDensity,
    pub default_quiet_uninstall: bool,
    pub uninstall_timeout_secs: u32,
}

impl Default for AppSettings {
    fn default() -> Self {
        AppSettings {
            show_system_components: false,
            show_32bit_programs: true,
            theme: Theme::System,
            confirm_before_uninstall: true,
            check_residue_after_uninstall: false,
            language: Language::ZhCn,
            list_density: ListDensity::Comfortable,
            default_quiet_uninstall: false,
            uninstall_timeout_secs: 300,
        }
    }
}

impl AppSettings {
    /// Clamps every field into its documented range, 技术文档 §3.4.
    pub fn normalise(mut self) -> Self {
        self.uninstall_timeout_secs = self.uninstall_timeout_secs.clamp(30, 3600);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub version: String,
    pub tauri_version: String,
    pub os: String,
    pub arch: String,
    pub log_dir: String,
    pub config_path: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_serialise_as_camel_case() {
        let json = serde_json::to_value(AppSettings::default()).unwrap();
        assert_eq!(json["showSystemComponents"], false);
        assert_eq!(json["uninstallTimeoutSecs"], 300);
        assert_eq!(json["theme"], "system");
        assert_eq!(json["language"], "zh-CN");
    }

    /// Pins every serialised key against the interfaces in `src/types/index.ts`.
    ///
    /// A key that does not match is not a visible typo: the webview reads
    /// `undefined`, `JSON.stringify` then drops the field, and the backend
    /// rejects the call complaining about a *missing* key. That is exactly how
    /// `show32bitPrograms` vs `show32BitPrograms` made the program list fail to
    /// load, so the whole key set is asserted rather than spot-checked.
    #[test]
    fn serialised_keys_match_the_typescript_contract() {
        let program = ProgramInfo {
            id: "HKLM:64:App".into(),
            key_name: "App".into(),
            reg_path: r"HKLM\SOFTWARE\...\Uninstall\App".into(),
            display_name: "App".into(),
            display_version: Some("1.0".into()),
            publisher: Some("Vendor".into()),
            install_date: Some("2026-01-01".into()),
            raw_install_date: Some("20260101".into()),
            install_location: Some(r"C:\Program Files\App".into()),
            estimated_size: Some(1024),
            uninstall_string: Some(r#"C:\a\un.exe"#.into()),
            quiet_uninstall_string: Some(r#"C:\a\un.exe" /S"#.into()),
            display_icon: Some("a.exe,0".into()),
            icon_path: Some(r"C:\a\a.exe".into()),
            system_component: false,
            windows_installer: true,
            parent_key_name: None,
            no_modify: false,
            no_repair: false,
            is_system: false,
            hive: Hive::HkLocalMachine,
            view: RegistryView::Bit64,
            is_64bit: true,
            can_uninstall: true,
        };

        let info = serde_json::to_value(&program).unwrap();
        let info_keys: Vec<&str> = info
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        for expected in [
            "id",
            "keyName",
            "regPath",
            "displayName",
            "displayVersion",
            "publisher",
            "installDate",
            "rawInstallDate",
            "installLocation",
            "estimatedSize",
            "uninstallString",
            "quietUninstallString",
            "displayIcon",
            "iconPath",
            "systemComponent",
            "windowsInstaller",
            "parentKeyName",
            "noModify",
            "noRepair",
            "isSystem",
            "hive",
            "view",
            "is64Bit",
            "canUninstall",
        ] {
            assert!(
                info_keys.contains(&expected),
                "ProgramInfo is missing the key `{expected}`; got {info_keys:?}"
            );
        }

        let settings = serde_json::to_value(AppSettings::default()).unwrap();
        let setting_keys: Vec<&str> = settings
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        for expected in [
            "showSystemComponents",
            "show32BitPrograms",
            "theme",
            "confirmBeforeUninstall",
            "checkResidueAfterUninstall",
            "language",
            "listDensity",
            "defaultQuietUninstall",
            "uninstallTimeoutSecs",
        ] {
            assert!(
                setting_keys.contains(&expected),
                "AppSettings is missing the key `{expected}`; got {setting_keys:?}"
            );
        }

        // Enum values are part of the same contract: the TS unions are
        // `'HKLM' | 'HKCU'` and `'64' | '32'`.
        assert_eq!(info["hive"], "HKLM");
        assert_eq!(info["view"], "64");
    }

    /// The renamed fields must round-trip from the exact JSON the webview sends.
    #[test]
    fn digit_bearing_keys_round_trip_from_the_frontend_shape() {
        let parsed: AppSettings = serde_json::from_str(r#"{"show32BitPrograms":false}"#).unwrap();
        assert!(!parsed.show_32bit_programs);

        let info: ProgramInfo = serde_json::from_str(
            r#"{"id":"x","keyName":"x","regPath":"x","displayName":"x","is64Bit":true,
                "systemComponent":false,"windowsInstaller":false,"noModify":false,
                "noRepair":false,"isSystem":false,"hive":"HKLM","view":"64",
                "canUninstall":true}"#,
        )
        .unwrap();
        assert!(info.is_64bit);
    }

    #[test]
    fn settings_missing_fields_fall_back_to_defaults() {
        // Simulates a settings.json written by an older build.
        let parsed: AppSettings = serde_json::from_str(r#"{"theme":"dark"}"#).unwrap();
        assert_eq!(parsed.theme, Theme::Dark);
        assert_eq!(parsed.language, Language::ZhCn);
        assert!(parsed.show_32bit_programs);
    }

    #[test]
    fn timeout_is_clamped() {
        let s = AppSettings {
            uninstall_timeout_secs: 5,
            ..Default::default()
        }
        .normalise();
        assert_eq!(s.uninstall_timeout_secs, 30);
        let s = AppSettings {
            uninstall_timeout_secs: 99999,
            ..Default::default()
        }
        .normalise();
        assert_eq!(s.uninstall_timeout_secs, 3600);
    }

    #[test]
    fn hive_parsing_is_lenient() {
        assert_eq!(Hive::parse("hklm"), Some(Hive::HkLocalMachine));
        assert_eq!(Hive::parse("HKEY_CURRENT_USER"), Some(Hive::HkCurrentUser));
        assert_eq!(Hive::parse("HKCR"), None);
    }
}
