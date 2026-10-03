use std::time::Instant;

use tauri::State;
use tracing::info;

use crate::error::{AppError, AppResult};
use crate::models::{ProgramInfo, ProgramsResponse};
use crate::registry::scanner::scan_all;
use crate::store::ProgramStore;

/// Full re-scan. Filtering by system/32-bit happens on the backend so the IPC
/// payload stays small; text search stays in the frontend.
///
/// Argument names are the camelCase forms of the Rust parameters. A word that
/// starts with a digit is renamed explicitly in `models.rs`; keep the same
/// discipline here so a JS key can never silently become `undefined`.
#[tauri::command]
pub async fn list_programs(
    store: State<'_, ProgramStore>,
    include_system: bool,
    include_x86: bool,
) -> AppResult<ProgramsResponse> {
    let started = Instant::now();
    let (programs, stats) =
        tauri::async_runtime::spawn_blocking(move || scan_all(include_system, include_x86))
            .await
            .map_err(|e| AppError::Internal(format!("扫描任务失败：{e}")))?;

    let elapsed_ms = started.elapsed().as_millis() as u64;
    store.replace_all(programs.clone());
    info!(
        count = programs.len(),
        elapsed_ms, include_system, include_x86, "list_programs"
    );

    Ok(ProgramsResponse {
        programs,
        elapsed_ms,
        stats,
    })
}

/// Looks a program up in the cache. Never touches the registry, so it is cheap.
#[tauri::command]
pub fn get_program(store: State<'_, ProgramStore>, id: String) -> AppResult<ProgramInfo> {
    store.get(&id)
}
