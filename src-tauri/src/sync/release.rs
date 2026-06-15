use rusqlite::Connection;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex};
use tauri::AppHandle;
use crate::auth;
use crate::db;
use crate::error::AppError;
use crate::rclone;
use crate::state::RunningSync;
use super::get_rclone_path;

/// Release a folder to cloud-only mode
/// 1. Sync to ensure cloud is up to date
/// 2. Delete local files
/// 3. Update folder mode to cloud_only
pub fn release_folder(
    app_handle: &AppHandle,
    active_syncs: &Arc<Mutex<HashMap<String, RunningSync>>>,
    conn: &Connection,
    app_data_dir: &Path,
    api_base_url: &str,
    folder_id: &str,
) -> Result<(), AppError> {
    let folder = db::folders::get_by_id(conn, folder_id)?;

    if folder.mode == "cloud_only" {
        return Err(AppError::General("Folder is already in cloud-only mode".into()));
    }

    // Get auth tokens (auto-refresh if expired)
    let tokens = auth::client::ensure_fresh_token(conn, api_base_url)?
        .ok_or_else(|| AppError::Auth("Not logged in. Please sign in again.".into()))?;

    // Fetch R2 credentials from backend
    let creds = auth::client::get_sync_credentials(
        api_base_url,
        &tokens.access_token,
        folder_id,
    )?;

    // Write rclone config and sync first
    let config_path = rclone::config::write_config(
        app_data_dir,
        &creds.endpoint,
        &creds.access_key_id,
        &creds.secret_access_key,
    )?;

    db::folders::update_status(conn, folder_id, "syncing")?;

    let rclone_path = get_rclone_path(Some(app_handle));
    let result = rclone::bisync::run_bisync(
        app_handle,
        active_syncs,
        folder_id,
        &rclone_path,
        &config_path,
        &folder.local_path,
        &creds.bucket,
        &creds.remote_path,
        creds.needs_resync,
    )?;

    // Clean up config
    let _ = std::fs::remove_file(&config_path);

    if !result.success {
        db::folders::update_status(conn, folder_id, "error")?;
        return Err(AppError::Rclone("Sync failed before release. Aborting.".into()));
    }

    // Delete local files (keep the directory itself)
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

    // Update folder mode
    db::folders::update_mode(conn, folder_id, "cloud_only")?;
    // Also mark the final sync as complete so the folder does not remain in `syncing` status.
    let _ = db::folders::update_last_sync(conn, folder_id);
    // Re-apply the cloud_only mode (update_last_sync sets status='synced', we want 'released').
    let _ = conn.execute(
        "UPDATE folders SET status = 'released' WHERE id = ?1",
        rusqlite::params![folder_id],
    );

    // Report to backend
    let report = auth::client::SyncReport {
        folder_id: folder_id.to_string(),
        success: true,
        files_transferred: result.files_transferred as u64,
        files_deleted: result.files_deleted as u64,
        duration_ms: result.duration_ms,
        errors: vec![],
        conflicts: vec![],
    };
    let _ = auth::client::report_sync(api_base_url, &tokens.access_token, &report);

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
