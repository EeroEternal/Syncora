use tauri::State;
use crate::db;
use crate::db::sync_logs::SyncLog;
use crate::error::AppError;
use crate::state::AppState;

#[tauri::command]
pub fn get_logs(
    state: State<AppState>,
    folder_id: Option<String>,
    limit: Option<i64>,
) -> Result<Vec<SyncLog>, AppError> {
    let conn = state.db.lock().unwrap();
    db::sync_logs::get_logs(&conn, folder_id.as_deref(), limit.unwrap_or(50))
}

#[tauri::command]
pub fn get_recent_activity(
    state: State<AppState>,
    limit: Option<i64>,
) -> Result<Vec<SyncLog>, AppError> {
    let conn = state.db.lock().unwrap();
    db::sync_logs::get_logs(&conn, None, limit.unwrap_or(10))
}
