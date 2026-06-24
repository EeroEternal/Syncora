// S3 sync engine for mobile (Android).
// Implements bidirectional sync using direct S3 API calls to Cloudflare R2,
// replacing the rclone subprocess used on desktop.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use rusqlite::Connection;
use tauri::{AppHandle, Emitter};
use log::{info, warn, error};

use crate::auth;
use crate::db;
use crate::error::AppError;
use crate::rclone::{SyncResult, ConflictInfo};
use crate::state::RunningSync;

pub mod types;
pub mod client;
pub mod algorithm;
pub mod localfs;

use client::S3Client;
use types::{FileEntry, SyncAction};

/// Run a bidirectional sync for a single folder using the S3 API.
///
/// This is the mobile equivalent of `sync::sync_folder` (desktop rclone path).
/// It:
/// 1. Loads folder metadata + R2 credentials from the backend
/// 2. Registers a RunningSync entry (cancel flag only, no subprocess)
/// 3. Marks the folder as "syncing"
/// 4. Runs the 3-way comparison algorithm (local vs remote vs last state)
/// 5. Reports results to the backend + updates local DB
#[cfg(target_os = "android")]
pub async fn run_sync(
    app_handle: AppHandle,
    active_syncs: Arc<Mutex<HashMap<String, RunningSync>>>,
    db: Arc<Mutex<Connection>>,
    _app_data_dir: PathBuf,
    api_base_url: String,
    folder_id: String,
) -> Result<SyncResult, AppError> {
    let start = Instant::now();
    let mut errors: Vec<String> = Vec::new();
    let mut conflicts: Vec<ConflictInfo> = Vec::new();
    let mut files_transferred: u32 = 0;
    let mut files_deleted: u32 = 0;
    let mut transferred_paths: Vec<String> = Vec::new();

    // ── Step 1: Load folder metadata ────────────────────────────────────
    let folder = {
        let conn = db.lock().unwrap();
        db::folders::get_by_id(&conn, &folder_id)?
    };

    if folder.mode == "cloud_only" {
        return Err(AppError::General("Folder is in cloud-only mode".into()));
    }

    // ── Step 2: Get auth tokens (blocking → spawn_blocking) ─────────────
    let api_url = api_base_url.clone();
    let db_clone = db.clone();
    let tokens = tokio::task::spawn_blocking(move || {
        let conn = db_clone.lock().unwrap();
        auth::client::ensure_fresh_token(&conn, &api_url)
    })
    .await
    .map_err(|e| AppError::General(e.to_string()))??
    .ok_or_else(|| AppError::Auth("Not logged in. Please sign in again.".into()))?;

    // ── Step 3: Fetch R2 credentials ────────────────────────────────────
    let api_url = api_base_url.clone();
    let token = tokens.access_token.clone();
    let fid = folder_id.clone();
    let creds = tokio::task::spawn_blocking(move || {
        auth::client::get_sync_credentials(&api_url, &token, &fid)
    })
    .await
    .map_err(|e| AppError::General(e.to_string()))??;

    // ── Step 4: Register RunningSync ────────────────────────────────────
    let cancel_flag = Arc::new(AtomicBool::new(false));
    let rs = RunningSync {
        cancel_requested: cancel_flag.clone(),
        started_at: Instant::now(),
    };
    {
        let mut map = active_syncs.lock().unwrap();
        map.insert(folder_id.clone(), rs);
    }

    // ── Step 5: Mark folder as syncing ──────────────────────────────────
    {
        let conn = db.lock().unwrap();
        db::folders::update_status(&conn, &folder_id, "syncing")?;
    }

    let _ = app_handle.emit("sync-status-changed", &folder_id);

    // ── Step 6: Walk local directory ────────────────────────────────────
    let local_path = folder.local_path.clone();
    let local_files = match localfs::walk_local_dir(&local_path) {
        Ok(files) => files,
        Err(e) => {
            error!("S3 sync: failed to walk local dir {}: {}", local_path, e);
            errors.push(format!("Failed to read local directory: {}", e));
            // Finalize with error
            finalize_sync(
                &db, &folder_id, &api_base_url, &tokens.access_token,
                false, 0, 0, 0, &errors, &conflicts, &transferred_paths,
                &active_syncs,
            ).await;
            return Ok(SyncResult {
                success: false,
                files_transferred: 0,
                files_deleted: 0,
                transferred_paths: Vec::new(),
                errors,
                conflicts,
                duration_ms: start.elapsed().as_millis() as u64,
            });
        }
    };

    // ── Step 7: List remote objects ─────────────────────────────────────
    let s3_client = S3Client::new(
        &creds.endpoint,
        &creds.access_key_id,
        &creds.secret_access_key,
        &creds.bucket,
    );

    let remote_prefix = creds.remote_path.clone();
    let remote_files = match s3_client.list_objects(&remote_prefix).await {
        Ok(files) => files,
        Err(e) => {
            error!("S3 sync: failed to list remote objects: {}", e);
            errors.push(format!("Failed to list remote objects: {}", e));
            finalize_sync(
                &db, &folder_id, &api_base_url, &tokens.access_token,
                false, 0, 0, 0, &errors, &conflicts, &transferred_paths,
                &active_syncs,
            ).await;
            return Ok(SyncResult {
                success: false,
                files_transferred: 0,
                files_deleted: 0,
                transferred_paths: Vec::new(),
                errors,
                conflicts,
                duration_ms: start.elapsed().as_millis() as u64,
            });
        }
    };

    // ── Step 8: Load last sync state ────────────────────────────────────
    let last_state = {
        let conn = db.lock().unwrap();
        db::sync_state::load_state(&conn, &folder_id)?
    };

    // ── Step 9: Compute sync actions ────────────────────────────────────
    let actions = if creds.needs_resync || folder.needs_resync || last_state.is_empty() {
        // First-time sync: upload all local files as source of truth
        let mut actions = HashMap::new();
        for path in local_files.keys() {
            actions.insert(path.clone(), SyncAction::UploadLocal);
        }
        // Delete remote files that don't exist locally (during resync, local is truth)
        for path in remote_files.keys() {
            if !local_files.contains_key(path) {
                actions.insert(path.clone(), SyncAction::DeleteRemote);
            }
        }
        actions
    } else {
        algorithm::compute_sync_actions(&local_files, &remote_files, &last_state)
    };

    // ── Step 10: Execute actions ────────────────────────────────────────
    let local_root = std::path::Path::new(&folder.local_path);

    for (rel_path, action) in &actions {
        // Check for cancel request
        if cancel_flag.load(Ordering::SeqCst) {
            info!("S3 sync: cancelled by user");
            errors.push("Sync cancelled by user".to_string());
            break;
        }

        let remote_key = format!("{}/{}", remote_prefix.trim_end_matches('/'), rel_path);

        match action {
            SyncAction::UploadLocal => {
                let local_file_path = local_root.join(rel_path);
                let data = match localfs::read_file(&local_file_path.to_string_lossy()) {
                    Ok(d) => d,
                    Err(e) => {
                        errors.push(format!("Failed to read {}: {}", rel_path, e));
                        continue;
                    }
                };

                // Emit progress
                let _ = app_handle.emit("sync-progress", serde_json::json!({
                    "folder_id": &folder_id,
                    "message": format!("Uploading {}", rel_path),
                }));

                match s3_client.put_object(&remote_key, data, None).await {
                    Ok(etag) => {
                        files_transferred += 1;
                        transferred_paths.push(rel_path.clone());
                        info!("S3 sync: uploaded {}", rel_path);
                    }
                    Err(e) => {
                        errors.push(format!("Failed to upload {}: {}", rel_path, e));
                    }
                }
            }
            SyncAction::DownloadRemote => {
                let local_file_path = local_root.join(rel_path);

                let _ = app_handle.emit("sync-progress", serde_json::json!({
                    "folder_id": &folder_id,
                    "message": format!("Downloading {}", rel_path),
                }));

                match s3_client.get_object(&remote_key).await {
                    Ok(data) => {
                        if let Err(e) = localfs::write_file(&local_file_path.to_string_lossy(), &data) {
                            errors.push(format!("Failed to write {}: {}", rel_path, e));
                            continue;
                        }
                        files_transferred += 1;
                        transferred_paths.push(rel_path.clone());
                        info!("S3 sync: downloaded {}", rel_path);
                    }
                    Err(e) => {
                        errors.push(format!("Failed to download {}: {}", rel_path, e));
                    }
                }
            }
            SyncAction::DeleteRemote => {
                match s3_client.delete_object(&remote_key).await {
                    Ok(_) => {
                        files_deleted += 1;
                        info!("S3 sync: deleted remote {}", rel_path);
                    }
                    Err(e) => {
                        errors.push(format!("Failed to delete remote {}: {}", rel_path, e));
                    }
                }
            }
            SyncAction::DeleteLocal => {
                let local_file_path = local_root.join(rel_path);
                if let Err(e) = localfs::delete_file(&local_file_path.to_string_lossy()) {
                    errors.push(format!("Failed to delete local {}: {}", rel_path, e));
                } else {
                    files_deleted += 1;
                    info!("S3 sync: deleted local {}", rel_path);
                }
            }
            SyncAction::Conflict => {
                let local_entry = local_files.get(rel_path);
                let remote_entry = remote_files.get(rel_path);
                conflicts.push(ConflictInfo {
                    file_path: rel_path.clone(),
                    local_version: local_entry.map(|e| format!("{} bytes", e.size)),
                    remote_version: remote_entry.map(|e| format!("{} bytes", e.size)),
                });
                warn!("S3 sync: conflict on {}", rel_path);
            }
            SyncAction::Skip => {}
        }
    }

    // ── Step 11: Build new sync state (local ∪ remote after sync) ───────
    let mut new_state: HashMap<String, FileEntry> = HashMap::new();
    for (path, entry) in &local_files {
        if !errors.iter().any(|e| e.contains(path)) {
            new_state.insert(path.clone(), entry.clone());
        }
    }
    for (path, entry) in &remote_files {
        if !new_state.contains_key(path) && !errors.iter().any(|e| e.contains(path)) {
            new_state.insert(path.clone(), entry.clone());
        }
    }

    // ── Step 12: Save sync state ────────────────────────────────────────
    {
        let conn = db.lock().unwrap();
        db::sync_state::save_state(&conn, &folder_id, &new_state)?;
    }

    let success = errors.is_empty();
    let duration_ms = start.elapsed().as_millis() as u64;

    // ── Step 13: Finalize (report to backend + update DB) ───────────────
    finalize_sync(
        &db, &folder_id, &api_base_url, &tokens.access_token,
        success, files_transferred, files_deleted, duration_ms,
        &errors, &conflicts, &transferred_paths, &active_syncs,
    ).await;

    Ok(SyncResult {
        success,
        files_transferred,
        files_deleted,
        transferred_paths,
        errors,
        conflicts,
        duration_ms,
    })
}

/// Desktop stub — run_sync is only called from the Android branch of sync_folder_async.
#[cfg(not(target_os = "android"))]
pub async fn run_sync(
    _app_handle: AppHandle,
    _active_syncs: Arc<Mutex<HashMap<String, RunningSync>>>,
    _db: Arc<Mutex<Connection>>,
    _app_data_dir: PathBuf,
    _api_base_url: String,
    _folder_id: String,
) -> Result<SyncResult, AppError> {
    Err(AppError::General(
        "S3 sync engine is only available on Android. Use the rclone path on desktop.".into(),
    ))
}

/// Helper: report sync result to backend, update local DB, and remove from active_syncs.
#[cfg(target_os = "android")]
async fn finalize_sync(
    db: &Arc<Mutex<Connection>>,
    folder_id: &str,
    api_base_url: &str,
    access_token: &str,
    success: bool,
    files_transferred: u32,
    files_deleted: u32,
    duration_ms: u64,
    errors: &[String],
    conflicts: &[ConflictInfo],
    transferred_paths: &[String],
    active_syncs: &Arc<Mutex<HashMap<String, RunningSync>>>,
) {
    // Remove from active_syncs
    {
        let mut map = active_syncs.lock().unwrap();
        map.remove(folder_id);
    }

    // Report to backend
    let report = auth::client::SyncReport {
        folder_id: folder_id.to_string(),
        success,
        files_transferred: files_transferred as u64,
        files_deleted: files_deleted as u64,
        duration_ms,
        errors: errors.to_vec(),
        conflicts: conflicts
            .iter()
            .map(|c| auth::client::ConflictReport {
                file_path: c.file_path.clone(),
                local_version: c.local_version.clone(),
                remote_version: c.remote_version.clone(),
            })
            .collect(),
    };

    let api_url = api_base_url.to_string();
    let token = access_token.to_string();
    let _ = tokio::task::spawn_blocking(move || {
        auth::client::report_sync(&api_url, &token, &report)
    }).await;

    // Update local DB
    {
        let conn = db.lock().unwrap();
        if success {
            let _ = db::folders::update_last_sync(&conn, folder_id);

            let summary = if files_transferred == 0 && files_deleted == 0 {
                "No changes".to_string()
            } else {
                let mut parts = Vec::new();
                if files_transferred > 0 {
                    parts.push(format!("{} synced", files_transferred));
                }
                if files_deleted > 0 {
                    parts.push(format!("{} deleted", files_deleted));
                }
                let summary_line = parts.join(", ");
                if !transferred_paths.is_empty() {
                    let names: Vec<&str> = transferred_paths
                        .iter()
                        .take(3)
                        .map(|p| p.rsplit('/').next().unwrap_or(p.as_str()))
                        .collect();
                    let tail = if transferred_paths.len() > 3 {
                        format!(" and {} more", transferred_paths.len() - 3)
                    } else {
                        String::new()
                    };
                    format!("{}: {}{}", summary_line, names.join(", "), tail)
                } else {
                    summary_line
                }
            };

            let _ = db::sync_logs::insert(
                &conn,
                &uuid::Uuid::new_v4().to_string(),
                folder_id,
                "s3_sync",
                "success",
                Some(&summary),
                Some(duration_ms as i64),
            );
        } else {
            let _ = db::folders::update_status(&conn, folder_id, "error");
            let error_msg = errors.first().map(|s| s.as_str()).unwrap_or("Unknown error");
            let _ = db::sync_logs::insert(
                &conn,
                &uuid::Uuid::new_v4().to_string(),
                folder_id,
                "s3_sync",
                "error",
                Some(error_msg),
                Some(duration_ms as i64),
            );
        }

        // Store conflicts
        for conflict in conflicts {
            let _ = db::conflicts::create(
                &conn,
                &uuid::Uuid::new_v4().to_string(),
                folder_id,
                &conflict.file_path,
                conflict.local_version.as_deref(),
                conflict.remote_version.as_deref(),
            );
        }
    }
}
