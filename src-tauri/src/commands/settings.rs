use tauri::{Manager, State};
use crate::db;
use crate::db::settings::Settings;
use crate::error::AppError;
use crate::rclone;
use crate::state::AppState;
use crate::sync::get_rclone_path;
use serde::Serialize;

#[derive(Serialize)]
pub struct ConnectionResult {
    pub success: bool,
    pub message: String,
}

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

#[tauri::command]
pub fn test_r2_connection(
    app_handle: tauri::AppHandle,
    state: State<AppState>,
) -> Result<ConnectionResult, AppError> {
    let conn = state.db.lock().unwrap();
    let settings = db::settings::get(&conn)?;

    if settings.r2_endpoint.is_empty() || settings.r2_access_key.is_empty() {
        return Ok(ConnectionResult {
            success: false,
            message: "Please fill in all R2 credentials first".to_string(),
        });
    }

    let app_data_dir = app_handle.path()
        .app_data_dir()
        .map_err(|e| AppError::General(e.to_string()))?;

    let config_path = rclone::config::write_config(
        &app_data_dir,
        &settings.r2_endpoint,
        &settings.r2_access_key,
        &settings.r2_secret,
    )?;

    let rclone_path = get_rclone_path();
    let (success, message) = rclone::bisync::test_connection(
        &rclone_path,
        &config_path,
        &settings.r2_bucket,
    )?;

    Ok(ConnectionResult { success, message })
}
