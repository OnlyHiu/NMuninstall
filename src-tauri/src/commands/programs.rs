use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};
use std::time::Instant;

use tauri::State;
use tracing::{info, trace};

use crate::error::{AppError, AppResult};
use crate::models::{ProgramInfo, ProgramsResponse};
use crate::registry::scanner::scan_all;
use crate::store::ProgramStore;

/// Rendered icon data URLs, keyed by program id.
///
/// Icons cannot change while the app is running and rendering one costs a PE
/// read plus a shell round trip, so a scroll that re-renders the same rows
/// must not pay for it twice. Failures are cached as well — they are the
/// common case for system components and re-probing them on every scroll tick
/// is pure waste.
type IconCache = Mutex<HashMap<String, Option<Arc<String>>>>;

fn icon_cache() -> &'static IconCache {
    static CACHE: OnceLock<IconCache> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// A poisoned cache only means an unrelated thread panicked while holding it.
/// The map is still structurally valid, so recover instead of turning a
/// cosmetic failure into a broken list.
fn lock_icon_cache() -> MutexGuard<'static, HashMap<String, Option<Arc<String>>>> {
    icon_cache()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

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

/// Renders one program's icon as a `data:image/png;base64,…` URL.
///
/// The frontend used to point `<img src>` straight at `iconPath`, which is a
/// PE or ICO container that WebView2 cannot decode — every icon failed and
/// the UI silently substituted a letter avatar. Rendering happens here
/// instead.
///
/// Returns `Ok(None)` for "this program has no usable icon", which is an
/// expected outcome and never an error. Icons are rendered at the shell's
/// native size; the list asks for 18–36 px and lets CSS scale down, which
/// looks better than any resample we could run here.
#[tauri::command]
pub async fn get_program_icon(
    store: State<'_, ProgramStore>,
    id: String,
) -> AppResult<Option<String>> {
    if let Some(cached) = lock_icon_cache().get(&id).cloned() {
        trace!(id = %id, "icon cache hit");
        return Ok(cached.map(|url| url.as_ref().clone()));
    }

    let path = store.get(&id)?.icon_path;
    let rendered = tauri::async_runtime::spawn_blocking(move || {
        crate::icons::icon_data_url(path.as_deref(), DEFAULT_ICON_PX)
    })
    .await
    .map_err(|e| AppError::Internal(format!("图标渲染任务失败：{e}")))?;

    trace!(
        id = %id,
        rendered = rendered.is_some(),
        "icon extracted"
    );
    lock_icon_cache().insert(id, rendered.clone().map(Arc::new));
    Ok(rendered)
}

/// Side length handed to the renderer. `ExtractIconExW` ignores the request
/// and returns `SM_CXICON`, so this is really just an upper bound on the
/// upscale step.
const DEFAULT_ICON_PX: u32 = 32;
