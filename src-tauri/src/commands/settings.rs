use std::path::{Path, PathBuf};
use std::sync::RwLock;

use tauri::{AppHandle, Emitter, Manager, State};
use tracing::info;

use crate::error::{AppError, AppResult};
use crate::models::{AppInfo, AppSettings};
use crate::settings;
use crate::store::ProgramStore;

/// `AppSettings` is tiny; a process-global copy keeps command signatures clean.
/// It is only ever written from `save_settings`, which is single-user.
static SETTINGS: RwLock<Option<AppSettings>> = RwLock::new(None);

fn app_config_dir(app: &AppHandle) -> AppResult<PathBuf> {
    app.path()
        .app_config_dir()
        .map_err(|e| AppError::SettingsReadFailed(e.to_string()))
}

pub fn current_settings() -> AppSettings {
    SETTINGS
        .read()
        .ok()
        .and_then(|g| g.clone())
        .unwrap_or_default()
}

fn set_cached(value: AppSettings) {
    match SETTINGS.write() {
        Ok(mut g) => *g = Some(value),
        Err(poisoned) => *poisoned.into_inner() = Some(value),
    }
}

/// Called once during startup, before any window becomes visible.
pub fn load_settings(app: &AppHandle) -> AppResult<AppSettings> {
    let dir = app_config_dir(app)?;
    let loaded = settings::load_from(&dir)?;
    set_cached(loaded.clone());
    info!(
        dir = %dir.display(),
        "settings loaded"
    );
    Ok(loaded)
}

#[tauri::command]
pub fn get_settings(app: AppHandle) -> AppResult<AppSettings> {
    match SETTINGS.read() {
        Ok(g) if g.is_some() => return Ok(g.clone().unwrap_or_default()),
        _ => {}
    }
    load_settings(&app)
}

#[tauri::command]
pub fn save_settings(app: AppHandle, settings: AppSettings) -> AppResult<AppSettings> {
    let dir = app_config_dir(&app)?;
    let saved = settings::save_to(&dir, &settings)?;
    set_cached(saved.clone());
    info!(timeout = saved.uninstall_timeout_secs, "settings updated");
    let _ = app.emit("settings://changed", &saved);
    Ok(saved)
}

#[tauri::command]
pub fn get_app_info(app: AppHandle) -> AppResult<AppInfo> {
    let config = app_config_dir(&app)?;
    let log_dir = app
        .path()
        .app_log_dir()
        .unwrap_or_else(|_| crate::logging::fallback_log_dir());
    Ok(AppInfo {
        version: app.package_info().version.to_string(),
        tauri_version: tauri::VERSION.to_string(),
        os: std::env::consts::OS.to_string(),
        arch: std::env::consts::ARCH.to_string(),
        log_dir: log_dir.to_string_lossy().to_string(),
        config_path: config.join("settings.json").to_string_lossy().to_string(),
    })
}

#[tauri::command]
pub fn open_log_dir(app: AppHandle) -> AppResult<()> {
    let dir = app
        .path()
        .app_log_dir()
        .map_err(|e| AppError::PathNotFound(e.to_string()))?;
    open_dir(&dir)
}

/// Opens the `InstallLocation` recorded in the registry for a program.
#[tauri::command]
pub fn open_install_location(id: String, store: State<'_, ProgramStore>) -> AppResult<()> {
    let program = store.get(&id)?;
    let location = program
        .install_location
        .ok_or_else(|| AppError::PathNotFound("该程序未声明安装位置".into()))?;
    let resolved = resolve(&location).ok_or_else(|| AppError::PathNotAllowed(location.clone()))?;
    open_dir(Path::new(&resolved))
}

/// Opens a residue directory. Re-validated: the path must exist, but unlike
/// `clean_residue` this only *views* it, so it is not restricted to the
/// deletion whitelist.
#[tauri::command]
pub fn open_path(path: String) -> AppResult<()> {
    let resolved = resolve(&path).ok_or_else(|| AppError::PathNotAllowed(path.clone()))?;
    open_dir(Path::new(&resolved))
}

fn resolve(path: &str) -> Option<String> {
    let expanded = crate::icons::expand_env_vars(path.trim().trim_matches('"'))?;
    let p = PathBuf::from(expanded);
    if p.is_absolute() {
        Some(p.to_string_lossy().to_string())
    } else {
        None
    }
}

fn open_dir(dir: &Path) -> AppResult<()> {
    if !dir.exists() {
        return Err(AppError::PathNotFound(dir.to_string_lossy().to_string()));
    }
    std::process::Command::new("explorer.exe")
        .arg(dir)
        .spawn()
        .map(|_| ())
        .map_err(|e| AppError::Internal(format!("无法打开目录：{e}")))
}
