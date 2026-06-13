use rusqlite::Connection;
use std::fs;
use std::path::Path;
use crate::db;
use crate::error::AppError;
use crate::rclone;
use super::get_rclone_path;

/// Release a folder to cloud-only mode
/// 1. Sync to ensure cloud is up to date
/// 2. Delete local files
/// 3. Update folder mode to cloud_only
pub fn release_folder(
    conn: &Connection,
    app_data_dir: &Path,
    folder_id: &str,
) -> Result<(), AppError> {
    let folder = db::folders::get_by_id(conn, folder_id)?;

    if folder.mode == "cloud_only" {
        return Err(AppError::General("Folder is already in cloud-only mode".into()));
    }

    let settings = db::settings::get(conn)?;
    if settings.r2_endpoint.is_empty() {
        return Err(AppError::Config("R2 credentials not configured".into()));
    }

    // Step 1: Write rclone config and sync first
    let config_path = rclone::config::write_config(
        app_data_dir,
        &settings.r2_endpoint,
        &settings.r2_access_key,
        &settings.r2_secret,
    )?;

    db::folders::update_status(conn, folder_id, "syncing")?;

    let rclone_path = get_rclone_path();
    let result = rclone::bisync::run_bisync(
        &rclone_path,
        &config_path,
        &folder.local_path,
        &settings.r2_bucket,
        &folder.remote_prefix,
        folder.needs_resync,
    )?;

    if !result.success {
        db::folders::update_status(conn, folder_id, "error")?;
        return Err(AppError::Rclone("Sync failed before release. Aborting.".into()));
    }

    // Step 2: Delete local files (keep the directory itself)
    let local_path = Path::new(&folder.local_path);
    if local_path.exists() {
        for entry in fs::read_dir(local_path)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                fs::remove_dir_all(&path)?;
            } else {
                fs::remove_file(&path)?;
            }
        }
    }

    // Step 3: Update folder mode
    db::folders::update_mode(conn, folder_id, "cloud_only")?;

    // Log the release
    db::sync_logs::insert(
        conn,
        &uuid::Uuid::new_v4().to_string(),
        folder_id,
        "release",
        "success",
        Some("Folder released to cloud-only mode"),
        Some(result.duration_ms as i64),
    )?;

    Ok(())
}
