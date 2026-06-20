use std::sync::atomic::Ordering;
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

    let result = tokio::task::spawn_blocking(move || {
        sync::sync_folder(
            &app_handle_clone,
            &active_syncs,
            &db,
            &app_data_dir,
            &api_base_url,
            &fid,
        )
    })
    .await
    .map_err(|e| AppError::General(e.to_string()))?;

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
/// Sets the cancel flag and immediately attempts to kill the rclone subprocess.
#[tauri::command]
pub async fn cancel_sync(
    state: State<'_, AppState>,
    folder_id: String,
) -> Result<(), AppError> {
    let (cancel_flag, child) = {
        let map = state.active_syncs.lock().unwrap();
        match map.get(&folder_id) {
            Some(rs) => (rs.cancel_requested.clone(), rs.child.clone()),
            None => {
                return Err(AppError::General(format!(
                    "No active sync for folder {}", folder_id
                )));
            }
        }
    };

    // Signal the running task to stop
    cancel_flag.store(true, Ordering::SeqCst);
    // Also attempt to kill the child directly for faster termination
    {
        let mut child_guard = child.lock().unwrap();
        if let Err(e) = child_guard.kill() {
            // InvalidInput just means the child already exited — ignore
            if e.kind() != std::io::ErrorKind::InvalidInput {
                log::warn!("Failed to kill rclone subprocess for {}: {}", folder_id, e);
            }
        }
    }

    log::info!("Cancel requested for folder {}", folder_id);
    Ok(())
}

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
