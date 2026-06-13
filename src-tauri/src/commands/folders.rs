use tauri::State;
use crate::db;
use crate::db::folders::Folder;
use crate::error::AppError;
use crate::state::AppState;

#[tauri::command]
pub fn list_folders(state: State<AppState>) -> Result<Vec<Folder>, AppError> {
    let conn = state.db.lock().unwrap();
    db::folders::list_all(&conn)
}

#[tauri::command]
pub fn add_folder(
    state: State<AppState>,
    local_path: String,
    remote_prefix: String,
) -> Result<Folder, AppError> {
    let conn = state.db.lock().unwrap();
    let id = uuid::Uuid::new_v4().to_string();
    db::folders::create(&conn, &id, &local_path, &remote_prefix)
}

#[tauri::command]
pub fn delete_folder(state: State<AppState>, id: String) -> Result<(), AppError> {
    let conn = state.db.lock().unwrap();
    db::folders::delete(&conn, &id)
}
