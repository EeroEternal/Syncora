use std::path::PathBuf;
use tauri::{Emitter, Manager, State};
use crate::error::AppError;
use crate::state::AppState;
use crate::sync;

#[tauri::command]
pub async fn trigger_sync(
    app_handle: tauri::AppHandle,
    state: State<'_, AppState>,
    folder_id: String,
) -> Result<(), AppError> {
    // Check if sync is already in progress for this folder
    {
        let map = state.active_syncs.lock().unwrap();
        if map.contains_key(&folder_id) {
            return Err(AppError::SyncInProgress(folder_id));
        }
    }

    let app_data_dir = app_handle.path()
        .app_data_dir()
        .map_err(|e| AppError::General(e.to_string()))?;

    let db = state.db.clone();
    let active_syncs = state.active_syncs.clone();
    let api_base_url = state.api_base_url.clone();
    let app_handle_clone = app_handle.clone();
    let fid = folder_id.clone();

    let result = sync::sync_folder_async(
        app_handle_clone,
        active_syncs,
        db,
        app_data_dir,
        api_base_url,
        fid,
    )
    .await;

    // Emit event for frontend
    let _ = app_handle.emit("sync-status-changed", &folder_id);

    result.map(|_| ())
}

#[tauri::command]
pub async fn trigger_sync_all(
    _app_handle: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    // Trigger the scheduler's sync-all cycle via sync_notify, exactly
    // like the tray menu "Sync All Now" does.  This ensures each folder
    // sync runs via execute_sync() which emits per-folder
    // "sync-status-changed" events so the frontend updates live.
    state.sync_notify.notify_one();
    Ok(())
}

/// Cancel a running sync for a given folder_id.
/// Sets the cancel flag and (on desktop) kills the rclone subprocess.
#[tauri::command]
pub async fn cancel_sync(
    state: State<'_, AppState>,
    folder_id: String,
) -> Result<(), AppError> {
    {
        let map = state.active_syncs.lock().unwrap();
        match map.get(&folder_id) {
            Some(rs) => rs.cancel(),
            None => {
                return Err(AppError::General(format!(
                    "No active sync for folder {}", folder_id
                )));
            }
        }
    }

    log::info!("Cancel requested for folder {}", folder_id);
    Ok(())
}

/// Release a folder's remote (desktop-only — uses rclone subprocess).
#[cfg(not(target_os = "android"))]
#[tauri::command]
pub async fn release_folder(
    app_handle: tauri::AppHandle,
    state: State<'_, AppState>,
    folder_id: String,
) -> Result<(), AppError> {
    let app_data_dir: PathBuf = app_handle.path()
        .app_data_dir()
        .map_err(|e| AppError::General(e.to_string()))?;

    let db = state.db.clone();
    let active_syncs = state.active_syncs.clone();
    let api_base_url = state.api_base_url.clone();
    let app_handle_clone = app_handle.clone();
    let fid = folder_id.clone();

    tokio::task::spawn_blocking(move || {
        let conn = db.lock().unwrap();
        sync::release::release_folder(
            &app_handle_clone,
            &active_syncs,
            &conn,
            &app_data_dir,
            &api_base_url,
            &fid,
        )
    })
    .await
    .map_err(|e| AppError::General(e.to_string()))??;

    let _ = app_handle.emit("sync-status-changed", &folder_id);
    Ok(())
}

/// Release folder stub (Android — not applicable on mobile).
#[cfg(target_os = "android")]
#[tauri::command]
pub async fn release_folder(
    _app_handle: tauri::AppHandle,
    _state: State<'_, AppState>,
    _folder_id: String,
) -> Result<(), AppError> {
    Err(AppError::General("Release folder is not supported on Android".into()))
}
