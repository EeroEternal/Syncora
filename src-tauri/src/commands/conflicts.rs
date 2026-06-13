use tauri::State;
use crate::db;
use crate::db::conflicts::Conflict;
use crate::error::AppError;
use crate::state::AppState;

#[tauri::command]
pub fn list_conflicts(
    state: State<AppState>,
    resolved: Option<bool>,
) -> Result<Vec<Conflict>, AppError> {
    let conn = state.db.lock().unwrap();
    db::conflicts::list(&conn, resolved)
}

#[tauri::command]
pub fn resolve_conflict(
    state: State<AppState>,
    id: String,
    resolution: String,
) -> Result<(), AppError> {
    let conn = state.db.lock().unwrap();
    db::conflicts::resolve(&conn, &id, &resolution)
}
