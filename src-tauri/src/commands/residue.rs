use tauri::State;
use tracing::{info, warn};

use crate::error::{AppError, AppResult};
use crate::models::{CleanResult, ResidueReport};
use crate::residue;
use crate::store::ProgramStore;

/// Detects leftovers for one program (F-301 ~ F-303).
#[tauri::command]
pub fn check_residue(store: State<'_, ProgramStore>, id: String) -> AppResult<ResidueReport> {
    let program = store.get(&id)?;
    let report = residue::detect(&program);
    info!(
        id = %id,
        registry = report.registry_keys.len(),
        dirs = report.install_dirs.len(),
        cleanable = report.cleanable_count,
        "residue detected"
    );
    Ok(report)
}

/// Deletes the leftovers the user explicitly selected (F-304).
///
/// The frontend only ever sends `ResidueItem.id` values; the concrete paths are
/// re-derived from the cache here and re-validated, so a tampered request
/// cannot reach an arbitrary path.
#[tauri::command]
pub fn clean_residue(
    store: State<'_, ProgramStore>,
    id: String,
    registry_keys: Vec<String>,
    directories: Vec<String>,
    confirmed: bool,
) -> AppResult<CleanResult> {
    if !confirmed {
        return Err(AppError::NotSupported);
    }
    let program = store.get(&id)?;
    let plan = residue::plan(&program, &registry_keys, &directories);

    if plan.registry_paths.is_empty() && plan.directories.is_empty() {
        warn!(id = %id, "clean requested with nothing selectable");
        return Ok(CleanResult::default());
    }
    for path in plan.registry_paths.iter().chain(plan.directories.iter()) {
        info!(id = %id, path, "user selected residue for deletion");
    }

    Ok(residue::clean(&plan))
}
