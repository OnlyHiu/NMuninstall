//! `tracing` setup: console plus a rolling log file.
//!
//! A log file that cannot be created must never stop the app from starting, so
//! failure degrades to console-only output.

use std::path::PathBuf;

use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::fmt::writer::BoxMakeWriter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, Layer};

/// Used only when Tauri's path resolver is unavailable. `app_log_dir()`
/// already returns the per-app log directory, so the normal path must come
/// from there — appending anything here would nest a second copy.
pub fn fallback_log_dir() -> PathBuf {
    std::env::temp_dir().join("NMUninstall").join("logs")
}

/// Builds the stderr layer. The filter is attached per layer (`with_filter`)
/// rather than to the registry, which keeps the layer stack homogeneous.
///
/// Generic over the subscriber because `Filtered<L, F, S>` is parameterised by
/// the subscriber it will be layered onto, so a shared closure would pin `S` to
/// whichever call site type-checked first.
fn console_layer<S>(filter: &EnvFilter) -> impl tracing_subscriber::Layer<S>
where
    S: tracing::Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a>,
{
    use tracing_subscriber::Layer as _;
    tracing_subscriber::fmt::layer()
        .with_target(true)
        .with_writer(std::io::stderr)
        .with_filter(filter.clone())
}

pub fn init(log_dir: PathBuf, debug: bool) -> PathBuf {
    let filter = EnvFilter::try_from_env("NMUNINSTALL_LOG").unwrap_or_else(|_| {
        if debug {
            EnvFilter::new("info,nmuninstall_lib=debug")
        } else {
            EnvFilter::new("info,nmuninstall_lib=info")
        }
    });

    let file = std::fs::create_dir_all(&log_dir).ok().and_then(|_| {
        RollingFileAppender::builder()
            .rotation(Rotation::DAILY)
            .max_log_files(7)
            .filename_prefix("nmuninstall")
            .filename_suffix("log")
            .build(&log_dir)
            .ok()
    });

    // A second call (e.g. from `open_log_dir`) must not panic, hence `let _`.
    match file {
        Some(appender) => {
            let file_layer = tracing_subscriber::fmt::layer()
                .with_ansi(false)
                .with_target(true)
                .with_writer(BoxMakeWriter::new(appender))
                .with_filter(filter.clone());
            let _ = tracing_subscriber::registry()
                .with(file_layer)
                .with(console_layer(&filter))
                .try_init();
        }
        None => {
            eprintln!("[nmuninstall] 无法创建日志文件，改用控制台输出");
            let _ = tracing_subscriber::registry()
                .with(console_layer(&filter))
                .try_init();
        }
    }

    log_dir
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fallback_log_dir_is_absolute_and_namespaced() {
        let p = fallback_log_dir();
        assert!(p.is_absolute());
        let tail: Vec<_> = p
            .components()
            .rev()
            .take(2)
            .map(|c| c.as_os_str().to_string_lossy().to_string())
            .collect();
        assert_eq!(tail, vec!["logs".to_string(), "NMUninstall".to_string()]);
    }
}
