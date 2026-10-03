//! NMUninstall — Windows program manager and uninstaller.

pub mod commands;
pub mod error;
pub mod icons;
pub mod logging;
pub mod models;
pub mod registry;
pub mod residue;
pub mod settings;
pub mod store;
pub mod uninstaller;

use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{Emitter, Manager, WindowEvent};

use crate::commands::settings::load_settings;
use crate::store::ProgramStore;

const TRAY_ID: &str = "nmuninstall-tray";

/// Fallback tray glyph, used only when the app icon cannot be decoded (a
/// corrupt or missing bundle icon). Windows renders a 1×1 stretched block,
/// which is far better than an invisible tray entry the user cannot find in
/// order to reach the 退出 item.
const TRAY_FALLBACK_RGBA: [u8; 4] = [0x00, 0x67, 0xC0, 0xFF];

pub fn run() {
    let debug = cfg!(debug_assertions);

    tauri::Builder::default()
        .plugin(tauri_plugin_clipboard_manager::init())
        .manage(ProgramStore::default())
        .setup(move |app| {
            let handle = app.handle().clone();

            // Logging first: everything after this point is recorded.
            // `app_log_dir()` is already the per-app directory.
            let log_dir = handle
                .path()
                .app_log_dir()
                .unwrap_or_else(|_| logging::fallback_log_dir());
            logging::init(log_dir, debug);

            // Settings must be cached before the first `uninstall_program` call.
            if let Err(e) = load_settings(&handle) {
                tracing::warn!(error = %e, "using default settings");
            }

            setup_tray(app)?;
            setup_window_events(&handle);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::programs::list_programs,
            commands::programs::get_program,
            commands::programs::get_program_icon,
            commands::uninstall::uninstall_program,
            commands::residue::check_residue,
            commands::residue::clean_residue,
            commands::settings::get_settings,
            commands::settings::save_settings,
            commands::settings::get_app_info,
            commands::settings::open_install_location,
            commands::settings::open_path,
            commands::settings::open_log_dir,
        ])
        .build(tauri::generate_context!())
        .expect("failed to build NMUninstall")
        .run(|_app, _event| {
            // Nothing to do on exit: closing the window hides to tray, and the
            // tray's 退出 item calls `AppHandle::exit` directly.
        });
}

fn setup_tray(app: &tauri::App) -> tauri::Result<()> {
    let rescan = MenuItem::with_id(app, "rescan", "重新扫描", true, None::<&str>)?;
    let show = MenuItem::with_id(app, "show", "显示主窗口", true, None::<&str>)?;
    let logs = MenuItem::with_id(app, "logs", "打开日志目录", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
    let menu = Menu::with_items(
        app,
        &[
            &show,
            &rescan,
            &logs,
            &PredefinedMenuItem::separator(app)?,
            &quit,
        ],
    )?;

    // The tray icon is the only way back into a window that was hidden, so it
    // has to be the real application icon. A 1×1 placeholder made the entry
    // effectively invisible and left users stuck with no way to reach 退出.
    let icon = app
        .default_window_icon()
        .cloned()
        .unwrap_or_else(|| tauri::image::Image::new_owned(TRAY_FALLBACK_RGBA.to_vec(), 1, 1));
    if icon.width() < 16 {
        tracing::warn!(
            width = icon.width(),
            height = icon.height(),
            "bundle icon is too small for a tray glyph"
        );
    }

    TrayIconBuilder::with_id(TRAY_ID)
        .icon(icon)
        .tooltip("NMUninstall")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "show" => show_main(app),
            "rescan" => {
                show_main(app);
                let _ = app.emit("app://rescan", ());
            }
            "logs" => {
                if let Err(e) = commands::settings::open_log_dir(app.clone()) {
                    tracing::warn!(error = %e, "cannot open log dir");
                }
            }
            "quit" => app.exit(0),
            other => tracing::debug!(id = other, "unhandled tray menu item"),
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main(tray.app_handle());
            }
        })
        .build(app)?;
    Ok(())
}

fn show_main(app: &tauri::AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.show();
        let _ = win.unminimize();
        let _ = win.set_focus();
    }
}

fn setup_window_events(app: &tauri::AppHandle) {
    let Some(win) = app.get_webview_window("main") else {
        return;
    };
    // The handler is `'static`, so it needs its own handle rather than a
    // borrow of the one used to register it.
    let handle = win.clone();
    win.on_window_event(move |event| {
        if let WindowEvent::CloseRequested { api, .. } = event {
            // 关闭窗口时真正隐藏到托盘。之前只调用了 `prevent_close()` 却
            // 没有隐藏窗口，点击 X 后界面纹丝不动，用户看到的就是
            // 「关不掉」——其实是「毫无反应」。
            if !commands::settings::current_settings().close_to_tray {
                return; // 允许关闭：最后一个窗口销毁后进程退出
            }
            api.prevent_close();
            if let Err(e) = handle.hide() {
                tracing::warn!(error = %e, "cannot hide the main window");
            }
        }
    });
}
