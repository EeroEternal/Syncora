use std::sync::atomic::Ordering;

use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager,
};

use crate::state::AppState;

pub fn setup_tray(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let open_i = MenuItem::with_id(app, "open", "Open Syncora", true, None::<&str>)?;
    let sync_i = MenuItem::with_id(app, "sync_all", "Sync All Now", true, None::<&str>)?;
    let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;

    let menu = Menu::with_items(app, &[&open_i, &sync_i, &quit_i])?;

    let _tray = TrayIconBuilder::new()
        .icon(app.default_window_icon().unwrap().clone())
        .menu(&menu)
        .tooltip("Syncora - File Sync")
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            "sync_all" => {
                let state = app.state::<AppState>();
                state.sync_notify.notify_one();
            }
            "quit" => {
                // Kill all active rclone subprocesses before exiting
                let state = app.state::<AppState>();
                let active_syncs = state.active_syncs.clone();
                let map = active_syncs.lock().unwrap();
                for (folder_id, rs) in map.iter() {
                    rs.cancel_requested.store(true, Ordering::SeqCst);
                    let _ = rs.child.lock().map(|mut c| {
                        if let Err(e) = c.kill() {
                            if e.kind() != std::io::ErrorKind::InvalidInput {
                                log::warn!("Failed to kill rclone for {} on quit: {}", folder_id, e);
                            }
                        }
                    });
                }
                drop(map);
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
        })
        .build(app)?;

    Ok(())
}
