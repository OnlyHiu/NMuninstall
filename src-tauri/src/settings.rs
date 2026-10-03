//! Settings persistence: `%APPDATA%\NMUninstall\settings.json`.
//!
//! Writes go to a sibling `.tmp` file first and are then renamed over the
//! target, which is atomic on NTFS. A corrupt file is preserved as
//! `settings.json.corrupt` rather than being overwritten.

use std::fs;
use std::path::{Path, PathBuf};

use tracing::{info, warn};

use crate::error::{AppError, AppResult};
use crate::models::AppSettings;

/// Resolves the settings path without touching Tauri's state manager, so it
/// can be unit tested.
pub fn config_path_for(app_config_dir: &Path) -> PathBuf {
    app_config_dir.join("settings.json")
}

pub fn load_from(dir: &Path) -> AppResult<AppSettings> {
    let path = config_path_for(dir);
    match fs::read_to_string(&path) {
        Ok(text) => match serde_json::from_str::<AppSettings>(&text) {
            Ok(s) => Ok(s.normalise()),
            Err(e) => {
                let backup = path.with_file_name("settings.json.corrupt");
                warn!(path = %path.display(), error = %e, "settings.json is corrupt, resetting");
                let _ = fs::rename(&path, &backup);
                Ok(AppSettings::default())
            }
        },
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(AppSettings::default()),
        Err(e) => {
            warn!(path = %path.display(), error = %e, "cannot read settings, using defaults");
            Ok(AppSettings::default())
        }
    }
}

pub fn save_to(dir: &Path, settings: &AppSettings) -> AppResult<AppSettings> {
    let normalised = settings.clone().normalise();
    fs::create_dir_all(dir).map_err(|e| AppError::SettingsWriteFailed(e.to_string()))?;
    let path = config_path_for(dir);
    let tmp = path.with_file_name("settings.json.tmp");
    let body = serde_json::to_vec_pretty(&normalised)
        .map_err(|e| AppError::SettingsWriteFailed(e.to_string()))?;
    fs::write(&tmp, body).map_err(|e| AppError::SettingsWriteFailed(e.to_string()))?;
    fs::rename(&tmp, &path).map_err(|e| {
        let _ = fs::remove_file(&tmp);
        AppError::SettingsWriteFailed(e.to_string())
    })?;
    info!(path = %path.display(), "settings saved");
    Ok(normalised)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_file_yields_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let s = load_from(dir.path()).unwrap();
        assert_eq!(s.uninstall_timeout_secs, 300);
        assert!(s.show_32bit_programs);
        assert!(!s.show_system_components);
    }

    #[test]
    fn round_trip_preserves_values() {
        let dir = tempfile::tempdir().unwrap();
        let d = dir.path().to_path_buf();
        let s = AppSettings {
            show_system_components: true,
            theme: crate::models::Theme::Dark,
            uninstall_timeout_secs: 120,
            ..Default::default()
        };
        let saved = save_to(&d, &s).unwrap();
        assert!(saved.show_system_components);
        let loaded = load_from(&d).unwrap();
        assert_eq!(loaded.theme, crate::models::Theme::Dark);
        assert_eq!(loaded.uninstall_timeout_secs, 120);
    }

    #[test]
    fn save_returns_normalised_values() {
        let dir = tempfile::tempdir().unwrap();
        let s = AppSettings {
            uninstall_timeout_secs: 1,
            ..Default::default()
        };
        let saved = save_to(dir.path(), &s).unwrap();
        assert_eq!(saved.uninstall_timeout_secs, 30);
        // The normalised value is what lands on disk.
        assert_eq!(load_from(dir.path()).unwrap().uninstall_timeout_secs, 30);
    }

    #[test]
    fn corrupt_file_is_preserved_and_reset() {
        let dir = tempfile::tempdir().unwrap();
        let d = dir.path().to_path_buf();
        fs::write(config_path_for(&d), b"{ this is not json").unwrap();
        let s = load_from(&d).unwrap();
        assert_eq!(s.uninstall_timeout_secs, 300, "falls back to defaults");
        assert!(
            d.join("settings.json.corrupt").exists(),
            "the damaged file is kept for inspection"
        );
    }

    #[test]
    fn partial_json_keeps_defaults_for_absent_fields() {
        let dir = tempfile::tempdir().unwrap();
        let d = dir.path().to_path_buf();
        fs::write(config_path_for(&d), br#"{"showSystemComponents":true}"#).unwrap();
        let s = load_from(&d).unwrap();
        assert!(s.show_system_components);
        assert!(s.show_32bit_programs, "absent field keeps its default");
    }

    #[test]
    fn no_temp_file_is_left_behind() {
        let dir = tempfile::tempdir().unwrap();
        let d = dir.path().to_path_buf();
        save_to(&d, &AppSettings::default()).unwrap();
        assert!(!d.join("settings.json.tmp").exists());
        assert!(d.join("settings.json").exists());
    }
}
