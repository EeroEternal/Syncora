pub mod error;
pub mod state;
pub mod db;
pub mod rclone;
pub mod sync;
pub mod commands;
pub mod auth;
pub mod s3sync;
pub mod foreground_service;

// Desktop-only modules
#[cfg(not(target_os = "android"))]
pub mod tray;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::Notify;
use state::AppState;
use tauri::Manager;

// Desktop-only imports
#[cfg(not(target_os = "android"))]
use tauri::WindowEvent;
#[cfg(not(target_os = "android"))]
use tauri_plugin_autostart::ManagerExt;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let mut builder = tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(
            tauri_plugin_log::Builder::default()
                .level(log::LevelFilter::Info)
                .build(),
        );

    // Desktop-only plugins
    #[cfg(not(target_os = "android"))]
    {
        builder = builder
            .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }))
            .plugin(tauri_plugin_autostart::init(
                tauri_plugin_autostart::MacosLauncher::LaunchAgent,
                None,
            ));
    }

    // Android-only plugins
    #[cfg(target_os = "android")]
    {
        builder = builder.plugin(foreground_service::init());
    }

    builder = builder.setup(|app| {
        // Initialize database
        let app_data_dir = app.path().app_data_dir()?;
        let conn = db::init_database(&app_data_dir)
            .expect("Failed to initialize database");

        // Read API base URL from settings
        let settings = db::settings::get(&conn).unwrap_or_default();
        let api_base_url = settings.api_base_url.clone();

        // Shared notify for immediate sync triggers
        let sync_notify = Arc::new(Notify::new());

        // Set up app state
        let active_syncs: Arc<Mutex<HashMap<String, state::RunningSync>>> =
            Arc::new(Mutex::new(HashMap::new()));

        app.manage(AppState {
            db: Arc::new(Mutex::new(conn)),
            active_syncs: active_syncs.clone(),
            api_base_url: api_base_url.clone(),
            sync_notify: sync_notify.clone(),
        });

        // Desktop-only: sync auto-start setting + system tray
        #[cfg(not(target_os = "android"))]
        {
            let autolaunch = app.autolaunch();
            if settings.auto_start {
                let _ = autolaunch.enable();
            } else {
                let _ = autolaunch.disable();
            }

            tray::setup_tray(app)?;
        }

        // Start background sync scheduler (shared — works on both platforms)
        sync::scheduler::start(
            app.handle().clone(),
            app.state::<AppState>().db.clone(),
            active_syncs,
            api_base_url,
            sync_notify,
            app_data_dir,
        );

        Ok(())
    })
    .invoke_handler(tauri::generate_handler![
        commands::auth::register,
        commands::auth::login,
        commands::auth::logout,
        commands::auth::get_auth_status,
        commands::settings::get_settings,
        commands::settings::save_settings,
        commands::folders::list_folders,
        commands::folders::add_folder,
        commands::folders::delete_folder,
        commands::folders::open_folder,
        commands::sync_cmd::trigger_sync,
        commands::sync_cmd::trigger_sync_all,
        commands::sync_cmd::cancel_sync,
        commands::sync_cmd::release_folder,
        commands::conflicts::list_conflicts,
        commands::conflicts::resolve_conflict,
        commands::logs::get_logs,
        commands::logs::get_recent_activity,
    ]);

    // Desktop-only: close-to-tray + cancel syncs on destroy
    #[cfg(not(target_os = "android"))]
    {
        builder = builder.on_window_event(|window, event| {
            match event {
                // Hide to tray instead of closing
                WindowEvent::CloseRequested { api, .. } => {
                    api.prevent_close();
                    let _ = window.hide();
                }
                // Cancel all active syncs on actual exit
                WindowEvent::Destroyed => {
                    let state = window.state::<AppState>();
                    let active_syncs = state.active_syncs.clone();
                    let map = active_syncs.lock().unwrap();
                    for (_folder_id, rs) in map.iter() {
                        rs.cancel();
                    }
                }
                _ => {}
            }
        });
    }

    builder
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
