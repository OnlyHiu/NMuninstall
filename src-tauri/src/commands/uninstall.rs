use std::time::Duration;

use tauri::{AppHandle, Emitter, State};
use tracing::info;

use crate::commands::settings::current_settings;
use crate::error::{AppError, AppResult};
use crate::models::{ProgramInfo, UninstallResult, UninstallStatus};
use crate::residue;
use crate::store::ProgramStore;
use crate::uninstaller::executor::{self, RunOutcome};
use crate::uninstaller::parser::{self, ParseError};

/// Progress phases pushed to the frontend over `uninstall://progress`.
#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct Progress<'a> {
    id: &'a str,
    phase: &'a str,
    message: &'a str,
}

fn emit(app: &AppHandle, id: &str, phase: &str, message: &str) {
    let _ = app.emit("uninstall://progress", Progress { id, phase, message });
}

/// Grace period before probing for residue, so a GUI uninstaller has time to
/// release its files.
const RESIDUE_SETTLE_SECS: u64 = 3;

/// Resolves the command, launches it, and optionally reports residue (F-201 ~ F-207).
#[tauri::command]
pub async fn uninstall_program(
    app: AppHandle,
    store: State<'_, ProgramStore>,
    id: String,
    quiet: bool,
    check_residue: bool,
) -> AppResult<UninstallResult> {
    let program: ProgramInfo = store.get(&id)?;

    let (parsed, quiet_fallback) = parser::choose_command(
        program.uninstall_string.as_deref(),
        program.quiet_uninstall_string.as_deref(),
        quiet,
    )
    .map_err(map_parse_error)?;

    if program.is_system {
        info!(id = %id, name = %program.display_name, "uninstalling a system component");
    }

    let timeout = Duration::from_secs(u64::from(current_settings().uninstall_timeout_secs));
    emit(&app, &id, "starting", "正在启动卸载程序");
    let outcome = executor::run(&parsed, timeout).await?;

    // The row leaves the list either way: on failure the refresh brings it back.
    store.remove(&program.id);

    let mut result = build_result(&parsed, &outcome, quiet_fallback);
    if quiet_fallback {
        result
            .message
            .push_str("。该程序未声明静默卸载命令，已回退为交互式卸载");
    }

    if check_residue {
        emit(&app, &id, "detecting_residue", "正在检查残留");
        if !outcome.waited {
            tokio::time::sleep(Duration::from_secs(RESIDUE_SETTLE_SECS)).await;
        }
        result.residue = Some(residue::detect(&program));
    }

    emit(&app, &id, "done", "完成");
    Ok(result)
}

fn build_result(
    parsed: &parser::ParsedCommand,
    outcome: &RunOutcome,
    quiet_fallback: bool,
) -> UninstallResult {
    let (status, message) = match outcome.status {
        UninstallStatus::Completed => (UninstallStatus::Completed, "卸载完成".to_string()),
        UninstallStatus::Started => (
            UninstallStatus::Started,
            "已启动卸载程序，请在其窗口中完成卸载，完成后点击刷新".to_string(),
        ),
        UninstallStatus::Failed => (
            UninstallStatus::Failed,
            match outcome.exit_code {
                Some(code) => format!("卸载器退出码 {code}，卸载可能未完成"),
                None => "卸载器异常终止，卸载可能未完成".to_string(),
            },
        ),
    };

    UninstallResult {
        status,
        message,
        executable: parsed.executable.clone(),
        args: parsed.args.clone(),
        exit_code: outcome.exit_code,
        waited: outcome.waited,
        duration_ms: outcome.duration.as_millis() as u64,
        quiet_fallback,
        residue: None,
    }
}

fn map_parse_error(e: ParseError) -> AppError {
    match e {
        ParseError::Empty | ParseError::NoExecutable(_) => AppError::NoUninstallString,
        other => AppError::ParseFailed(other.to_string()),
    }
}
