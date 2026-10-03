//! Launches a parsed uninstall command and reports what happened.
//!
//! Two very different outcomes are possible:
//! - `Completed` — a waitable process (msiexec / rundll32) ran to the end.
//! - `Started` — a GUI uninstaller was launched and is not waited on.

use std::path::Path;
use std::time::{Duration, Instant};

use tracing::{info, warn};

use crate::error::{AppError, AppResult};
use crate::models::UninstallStatus;
use crate::uninstaller::parser::ParsedCommand;

const CREATE_NO_WINDOW: u32 = 0x0800_0000;
const DETACHED_PROCESS: u32 = 0x0000_0008;
const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;

pub struct RunOutcome {
    pub status: UninstallStatus,
    pub exit_code: Option<i32>,
    pub waited: bool,
    pub duration: Duration,
}

pub async fn run(parsed: &ParsedCommand, timeout: Duration) -> AppResult<RunOutcome> {
    let started = Instant::now();
    let exe = Path::new(&parsed.executable);

    let mut cmd = tokio::process::Command::new(&parsed.executable);
    cmd.args(&parsed.args).kill_on_drop(true);

    // A console window must never flash on screen.
    let mut flags = CREATE_NO_WINDOW;
    if !parsed.can_wait {
        // Detach so the uninstaller survives this process and is not tied to
        // our console session.
        flags |= DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP;
    }
    cmd.creation_flags(flags);

    // Some uninstallers resolve sibling files relative to the CWD.
    if let Some(dir) = exe.parent() {
        if dir.is_dir() {
            cmd.current_dir(dir);
        }
    }

    if !parsed.can_wait {
        info!(
            executable = %parsed.executable,
            args = ?parsed.args,
            "launching uninstaller without waiting"
        );
        cmd.spawn()
            .map_err(|e| AppError::SpawnFailed(format!("{e}（{}）", parsed.executable)))?;
        return Ok(RunOutcome {
            status: UninstallStatus::Started,
            exit_code: None,
            waited: false,
            duration: started.elapsed(),
        });
    }

    info!(
        executable = %parsed.executable,
        args = ?parsed.args,
        timeout_secs = timeout.as_secs(),
        "launching waitable uninstaller"
    );
    let mut child = cmd
        .spawn()
        .map_err(|e| AppError::SpawnFailed(format!("{e}（{}）", parsed.executable)))?;

    match tokio::time::timeout(timeout, child.wait()).await {
        Ok(Ok(status)) => {
            let code = status.code();
            let ok = status.success();
            if !ok {
                warn!(exit_code = ?code, "uninstaller reported failure");
            }
            Ok(RunOutcome {
                status: if ok {
                    UninstallStatus::Completed
                } else {
                    UninstallStatus::Failed
                },
                exit_code: code,
                waited: true,
                duration: started.elapsed(),
            })
        }
        Ok(Err(e)) => Err(AppError::SpawnFailed(e.to_string())),
        Err(_) => {
            // Timed out: reap the child so no zombie is left behind.
            warn!(
                timeout_secs = timeout.as_secs(),
                "uninstall timed out, killing child"
            );
            let _ = child.start_kill();
            let _ = child.wait().await;
            Err(AppError::UninstallTimeout(timeout.as_secs()))
        }
    }
}
