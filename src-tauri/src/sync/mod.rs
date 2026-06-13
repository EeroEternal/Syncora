pub mod release;

use std::path::PathBuf;
use crate::db;
use crate::error::AppError;
use crate::rclone;
use rusqlite::Connection;

/// Get the rclone binary path (sidecar)
pub fn get_rclone_path() -> PathBuf {
    // In development, look for rclone in PATH or in the binaries directory
    // In production (bundled), the sidecar will be resolved by Tauri
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."));

    // Try sidecar location first
    let sidecar = exe_dir.join("rclone");
    if sidecar.exists() {
        return sidecar;
    }

    // Fall back to system rclone
    PathBuf::from("rclone")
}

/// Execute sync for a single folder
pub fn sync_folder(
    conn: &Connection,
    app_data_dir: &std::path::Path,
    folder_id: &str,
) -> Result<rclone::SyncResult, AppError> {
    let folder = db::folders::get_by_id(conn, folder_id)?;

    if folder.mode == "cloud_only" {
        return Err(AppError::General("Folder is in cloud-only mode".into()));
    }

    // Get settings for R2 config
    let settings = db::settings::get(conn)?;
    if settings.r2_endpoint.is_empty() || settings.r2_access_key.is_empty() {
        return Err(AppError::Config("R2 credentials not configured".into()));
    }

    // Write rclone config
    let config_path = rclone::config::write_config(
        app_data_dir,
        &settings.r2_endpoint,
        &settings.r2_access_key,
        &settings.r2_secret,
    )?;

    // Update status to syncing
    db::folders::update_status(conn, folder_id, "syncing")?;

    // Run bisync
    let rclone_path = get_rclone_path();
    let result = rclone::bisync::run_bisync(
        &rclone_path,
        &config_path,
        &folder.local_path,
        &settings.r2_bucket,
        &folder.remote_prefix,
        folder.needs_resync,
    )?;

    // Process results
    if result.success {
        db::folders::update_last_sync(conn, folder_id)?;

        // Log success
        db::sync_logs::insert(
            conn,
            &uuid::Uuid::new_v4().to_string(),
            folder_id,
            "bisync",
            "success",
            Some("Sync completed successfully"),
            Some(result.duration_ms as i64),
        )?;
    } else {
        db::folders::update_status(conn, folder_id, "error")?;

        let error_msg = result.errors.first().map(|s| s.as_str()).unwrap_or("Unknown error");
        db::sync_logs::insert(
            conn,
            &uuid::Uuid::new_v4().to_string(),
            folder_id,
            "bisync",
            "error",
            Some(error_msg),
            Some(result.duration_ms as i64),
        )?;
    }

    // Handle conflicts
    for conflict in &result.conflicts {
        db::conflicts::create(
            conn,
            &uuid::Uuid::new_v4().to_string(),
            folder_id,
            &conflict.file_path,
            conflict.local_version.as_deref(),
            conflict.remote_version.as_deref(),
        )?;
    }

    Ok(result)
}
