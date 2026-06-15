pub mod release;
pub mod scheduler;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use crate::auth;
use crate::db;
use crate::error::AppError;
use crate::rclone;
use crate::state::RunningSync;
use rusqlite::Connection;

/// Resolve the rclone binary path.
///
/// Search order:
/// 1. Platform-suffixed sidecar next to the executable (`rclone-<triple>`) — production bundles
/// 2. Plain `rclone` next to the executable
/// 3. Walk up from executable to find `src-tauri/binaries/rclone-<triple>` (dev builds)
/// 4. Fallback to `rclone` on PATH (warn, may be wrong version)
pub fn get_rclone_path(_app_handle: Option<&tauri::AppHandle>) -> PathBuf {
    let target_triple = std::env::consts::ARCH;
    let os = std::env::consts::OS;
    let ext = if os == "windows" { ".exe" } else { "" };
    let triple_suffix = match (target_triple, os) {
        ("aarch64", "macos") => "aarch64-apple-darwin",
        ("x86_64", "macos")  => "x86_64-apple-darwin",
        ("x86_64", "linux")  => "x86_64-unknown-linux-gnu",
        ("aarch64", "linux") => "aarch64-unknown-linux-gnu",
        ("x86_64", "windows") => "x86_64-pc-windows-msvc",
        ("aarch64", "windows") => "aarch64-pc-windows-msvc",
        _ => "",
    };
    let sidecar_name = format!("rclone-{}{}", triple_suffix, ext);
    let plain_name = format!("rclone{}", ext);

    let is_valid = |p: &PathBuf| p.exists() && p.metadata().map(|m| m.len() > 0).unwrap_or(false);

    // 1. Sidecar next to the executable (production bundles place externalBin here)
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()));

    if let Some(ref dir) = exe_dir {
        let suffixed = dir.join(&sidecar_name);
        if is_valid(&suffixed) {
            return suffixed;
        }
        let plain = dir.join(&plain_name);
        if is_valid(&plain) {
            return plain;
        }
    }

    // 2. Dev-mode walk up from target/debug/syncora to find src-tauri/binaries/
    if let Some(exe) = std::env::current_exe().ok() {
        let mut dir = exe.parent();
        while let Some(d) = dir {
            let candidate = d.join("binaries").join(&sidecar_name);
            if is_valid(&candidate) {
                return candidate;
            }
            dir = d.parent();
        }
    }

    // 3. Last resort: hope rclone is on PATH
    log::warn!("Could not locate bundled rclone binary; falling back to system PATH");
    PathBuf::from(&plain_name)
}

/// Execute sync for a single folder.
/// The DB lock is held only during brief read/write operations;
/// it is released before HTTP calls and rclone execution so the UI
/// remains responsive while a sync is in progress.
pub fn sync_folder(
    app_handle: &tauri::AppHandle,
    active_syncs: &Arc<Mutex<HashMap<String, RunningSync>>>,
    db: &Arc<Mutex<Connection>>,
    app_data_dir: &std::path::Path,
    api_base_url: &str,
    folder_id: &str,
) -> Result<rclone::SyncResult, AppError> {
    // ── Step 1: Read folder metadata (brief lock) ──────────────────────────
    let folder = {
        let conn = db.lock().unwrap();
        db::folders::get_by_id(&conn, folder_id)?
    };

    if folder.mode == "cloud_only" {
        return Err(AppError::General("Folder is in cloud-only mode".into()));
    }

    // ── Step 2: Get / refresh auth tokens (brief lock; HTTP only if expired) ─
    let tokens = {
        let conn = db.lock().unwrap();
        auth::client::ensure_fresh_token(&conn, api_base_url)?
            .ok_or_else(|| AppError::Auth("Not logged in. Please sign in again.".into()))?
    };

    // ── Step 3: Fetch R2 credentials from backend (HTTP, no lock) ─────────
    let creds = auth::client::get_sync_credentials(
        api_base_url,
        &tokens.access_token,
        folder_id,
    )?;

    // ── Step 4: Write temporary rclone config (no lock needed) ────────────
    let config_path = rclone::config::write_config(
        app_data_dir,
        &creds.endpoint,
        &creds.access_key_id,
        &creds.secret_access_key,
    )?;

    // ── Step 5: Mark folder as syncing (brief lock) ───────────────────────
    {
        let conn = db.lock().unwrap();
        db::folders::update_status(&conn, folder_id, "syncing")?;
    }

    // ── Step 6: Run bisync (no lock – can take a long time) ───────────────
    let rclone_path = get_rclone_path(Some(app_handle));
    log::info!("Using rclone binary: {:?} (size: {} bytes)",
        rclone_path,
        std::fs::metadata(&rclone_path).map(|m| m.len()).unwrap_or(0)
    );
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
    );

    // Remove credentials file immediately after bisync
    let _ = std::fs::remove_file(&config_path);

    let result = result?;

    // ── Step 7: Report sync result to backend (HTTP, no lock) ─────────────
    let report = auth::client::SyncReport {
        folder_id: folder_id.to_string(),
        success: result.success,
        files_transferred: result.files_transferred as u64,
        files_deleted: result.files_deleted as u64,
        duration_ms: result.duration_ms,
        errors: result.errors.clone(),
        conflicts: result
            .conflicts
            .iter()
            .map(|c| auth::client::ConflictReport {
                file_path: c.file_path.clone(),
                local_version: c.local_version.clone(),
                remote_version: c.remote_version.clone(),
            })
            .collect(),
    };
    let _ = auth::client::report_sync(api_base_url, &tokens.access_token, &report);

    // ── Step 8: Update local DB with outcome (brief lock) ─────────────────
    {
        let conn = db.lock().unwrap();
        if result.success {
            db::folders::update_last_sync(&conn, folder_id)?;

            // Build a human-readable summary message
            let summary = if result.files_transferred == 0 && result.files_deleted == 0 {
                "No changes".to_string()
            } else {
                let mut parts = Vec::new();
                if result.files_transferred > 0 {
                    parts.push(format!("{} synced", result.files_transferred));
                }
                if result.files_deleted > 0 {
                    parts.push(format!("{} deleted", result.files_deleted));
                }
                let summary_line = parts.join(", ");
                if !result.transferred_paths.is_empty() {
                    let names: Vec<&str> = result
                        .transferred_paths
                        .iter()
                        .take(3)
                        .map(|p| p.rsplit('/').next().unwrap_or(p.as_str()))
                        .collect();
                    let tail = if result.transferred_paths.len() > 3 {
                        format!(" and {} more", result.transferred_paths.len() - 3)
                    } else {
                        String::new()
                    };
                    format!("{}: {}{}", summary_line, names.join(", "), tail)
                } else {
                    summary_line
                }
            };

            db::sync_logs::insert(
                &conn,
                &uuid::Uuid::new_v4().to_string(),
                folder_id,
                "bisync",
                "success",
                Some(&summary),
                Some(result.duration_ms as i64),
            )?;
        } else {
            db::folders::update_status(&conn, folder_id, "error")?;
            let error_msg = result.errors.first().map(|s| s.as_str()).unwrap_or("Unknown error");
            db::sync_logs::insert(
                &conn,
                &uuid::Uuid::new_v4().to_string(),
                folder_id,
                "bisync",
                "error",
                Some(error_msg),
                Some(result.duration_ms as i64),
            )?;
        }

        // Store conflicts locally
        for conflict in &result.conflicts {
            db::conflicts::create(
                &conn,
                &uuid::Uuid::new_v4().to_string(),
                folder_id,
                &conflict.file_path,
                conflict.local_version.as_deref(),
                conflict.remote_version.as_deref(),
            )?;
        }
    }

    Ok(result)
}
