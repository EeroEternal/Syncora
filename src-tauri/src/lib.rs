pub mod error;
pub mod state;
pub mod db;
pub mod rclone;
pub mod sync;
pub mod commands;
pub mod tray;

use std::collections::HashMap;
use std::sync::Mutex;
use state::AppState;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(
            tauri_plugin_log::Builder::default()
                .level(log::LevelFilter::Info)
                .build(),
        )
        .setup(|app| {
            // Initialize database
            let app_data_dir = app.path().app_data_dir()?;
            let conn = db::init_database(&app_data_dir)
                .expect("Failed to initialize database");

            // Set up app state
            app.manage(AppState {
                db: Mutex::new(conn),
                sync_locks: Mutex::new(HashMap::new()),
            });

            // Set up system tray
            tray::setup_tray(app)?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::settings::get_settings,
            commands::settings::save_settings,
            commands::settings::test_r2_connection,
            commands::folders::list_folders,
            commands::folders::add_folder,
            commands::folders::delete_folder,
            commands::sync_cmd::trigger_sync,
            commands::sync_cmd::trigger_sync_all,
            commands::sync_cmd::release_folder,
            commands::conflicts::list_conflicts,
            commands::conflicts::resolve_conflict,
            commands::logs::get_logs,
            commands::logs::get_recent_activity,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
