use tauri::{Emitter, Manager, State};
use crate::error::AppError;
use crate::state::AppState;
use crate::sync;

#[tauri::command]
pub fn trigger_sync(
    app_handle: tauri::AppHandle,
    state: State<AppState>,
    folder_id: String,
) -> Result<(), AppError> {
    // Check if sync is already in progress for this folder
    {
        let locks = state.sync_locks.lock().unwrap();
        if locks.get(&folder_id).copied().unwrap_or(false) {
            return Err(AppError::SyncInProgress(folder_id));
        }
    }

    // Set lock
    {
        let mut locks = state.sync_locks.lock().unwrap();
        locks.insert(folder_id.clone(), true);
    }

    let app_data_dir = app_handle.path()
        .app_data_dir()
        .map_err(|e| AppError::General(e.to_string()))?;

    let conn = state.db.lock().unwrap();
    let result = sync::sync_folder(&conn, &app_data_dir, &folder_id);

    // Release lock
    {
        let mut locks = state.sync_locks.lock().unwrap();
        locks.insert(folder_id.clone(), false);
    }

    // Emit event for frontend
    let _ = app_handle.emit("sync-status-changed", &folder_id);

    result.map(|_| ())
}

#[tauri::command]
pub fn trigger_sync_all(
    app_handle: tauri::AppHandle,
    state: State<AppState>,
) -> Result<(), AppError> {
    let app_data_dir = app_handle.path()
        .app_data_dir()
        .map_err(|e| AppError::General(e.to_string()))?;

    let conn = state.db.lock().unwrap();
    let folders = crate::db::folders::list_all(&conn)?;

    for folder in folders {
        if folder.is_enabled && folder.mode != "cloud_only" {
            let _ = sync::sync_folder(&conn, &app_data_dir, &folder.id);
        }
    }

    let _ = app_handle.emit("sync-status-changed", "all");
    Ok(())
}

#[tauri::command]
pub fn release_folder(
    app_handle: tauri::AppHandle,
    state: State<AppState>,
    folder_id: String,
) -> Result<(), AppError> {
    let app_data_dir = app_handle.path()
        .app_data_dir()
        .map_err(|e| AppError::General(e.to_string()))?;

    let conn = state.db.lock().unwrap();
    sync::release::release_folder(&conn, &app_data_dir, &folder_id)?;

    let _ = app_handle.emit("sync-status-changed", &folder_id);
    Ok(())
}
