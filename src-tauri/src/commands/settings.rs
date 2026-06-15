use tauri::State;
use crate::db;
use crate::db::settings::Settings;
use crate::error::AppError;
use crate::state::AppState;

#[tauri::command]
pub fn get_settings(state: State<AppState>) -> Result<Settings, AppError> {
    let conn = state.db.lock().unwrap();
    db::settings::get(&conn)
}

#[tauri::command]
pub fn save_settings(state: State<AppState>, settings: Settings) -> Result<(), AppError> {
    let conn = state.db.lock().unwrap();
    db::settings::save(&conn, &settings)
}
