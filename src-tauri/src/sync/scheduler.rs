use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[cfg(not(target_os = "android"))]
use notify_debouncer_mini::{new_debouncer, DebouncedEventKind};
use tokio::sync::{mpsc, Notify};
use tauri::{AppHandle, Emitter};
use log::{info, warn, error};
use rusqlite::Connection;

use crate::db;
use crate::auth;
use crate::state::RunningSync;
use crate::sync;

/// Message sent internally to trigger sync for specific folders.
enum SyncTrigger {
    /// Sync specific folders (triggered by file watcher or remote polling)
    Folders(Vec<String>),
    /// Sync all enabled folders (startup or tray "Sync All")
    All,
}

/// Start the background sync system:
/// 1. File watcher: monitors local folders, triggers sync on changes (debounced 3s)
/// 2. Remote poller: checks backend every 30s for remote updates
/// 3. Startup: syncs all folders once on launch
pub fn start(
    app_handle: AppHandle,
    db: Arc<Mutex<Connection>>,
    active_syncs: Arc<Mutex<HashMap<String, RunningSync>>>,
    api_base_url: String,
    sync_notify: Arc<Notify>,
    app_data_dir: PathBuf,
) {
    // Channel for sync triggers from watcher/poller → executor
    let (tx, rx) = mpsc::unbounded_channel::<SyncTrigger>();

    // Spawn file watcher thread (desktop-only — Android uses remote polling)
    #[cfg(not(target_os = "android"))]
    start_file_watcher(db.clone(), tx.clone());

    // Spawn remote poller task
    start_remote_poller(db.clone(), api_base_url.clone(), tx.clone());

    // Spawn sync executor task
    start_sync_executor(
        app_handle,
        db,
        active_syncs,
        api_base_url,
        app_data_dir,
        rx,
        sync_notify,
        tx,
    );
}

/// File watcher: uses `notify` to watch all registered local folders.
/// Sends folder IDs to the sync channel when files change (debounced 3s).
#[cfg(not(target_os = "android"))]
fn start_file_watcher(
    db: Arc<Mutex<Connection>>,
    tx: mpsc::UnboundedSender<SyncTrigger>,
) {
    std::thread::spawn(move || {
        // Map: watched path → folder_id
        let mut path_to_folder: HashMap<PathBuf, String> = HashMap::new();

        let tx_clone = tx.clone();
        let (notify_tx, notify_rx) = std::sync::mpsc::channel();

        // Create debouncer with 3s debounce timeout
        let mut debouncer = match new_debouncer(Duration::from_secs(3), notify_tx) {
            Ok(d) => d,
            Err(e) => {
                error!("Failed to create file watcher: {}", e);
                return;
            }
        };

        // Initial setup: watch all existing folders
        {
            let conn = db.lock().unwrap();
            if let Ok(folders) = db::folders::list_all(&conn) {
                for folder in folders {
                    if folder.is_enabled && folder.mode != "cloud_only" {
                        let path = PathBuf::from(&folder.local_path);
                        if path.exists() {
                            if let Err(e) = debouncer.watcher().watch(
                                &path,
                                notify::RecursiveMode::Recursive,
                            ) {
                                warn!("Failed to watch {}: {}", folder.local_path, e);
                            } else {
                                info!("Watching folder: {}", folder.local_path);
                                path_to_folder.insert(path, folder.id);
                            }
                        }
                    }
                }
            }
        }

        // Process debounced events
        loop {
            match notify_rx.recv() {
                Ok(Ok(events)) => {
                    let mut triggered_folders: HashMap<String, bool> = HashMap::new();

                    for event in events {
                        if event.kind == DebouncedEventKind::Any {
                            // Find which folder this path belongs to
                            for (watched_path, folder_id) in &path_to_folder {
                                if event.path.starts_with(watched_path) {
                                    triggered_folders.insert(folder_id.clone(), true);
                                    break;
                                }
                            }
                        }
                    }

                    if !triggered_folders.is_empty() {
                        let folder_ids: Vec<String> = triggered_folders.into_keys().collect();
                        info!("File changes detected in folders: {:?}", folder_ids);
                        let _ = tx_clone.send(SyncTrigger::Folders(folder_ids));
                    }
                }
                Ok(Err(errs)) => {
                    warn!("File watcher errors: {:?}", errs);
                }
                Err(_) => {
                    // Channel closed, exit
                    break;
                }
            }
        }
    });
}

/// Remote poller: every 30s, checks the backend for folders updated by other devices.
fn start_remote_poller(
    db: Arc<Mutex<Connection>>,
    api_base_url: String,
    tx: mpsc::UnboundedSender<SyncTrigger>,
) {
    tauri::async_runtime::spawn(async move {
        // Track last check time
        let mut last_check = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S").to_string();

        loop {
            tokio::time::sleep(Duration::from_secs(30)).await;

            // Run all blocking HTTP calls inside spawn_blocking
            let db_clone = db.clone();
            let api_url = api_base_url.clone();
            let since = last_check.clone();

            let result = tokio::task::spawn_blocking(move || -> Result<Vec<String>, crate::error::AppError> {
                // Get fresh token
                let conn = db_clone.lock().unwrap();
                let tokens = match auth::client::ensure_fresh_token(&conn, &api_url) {
                    Ok(Some(t)) => t,
                    _ => return Ok(Vec::new()),
                };
                drop(conn);

                // Check for remote updates
                let updated = auth::client::check_remote_updates(
                    &api_url,
                    &tokens.access_token,
                    &since,
                )?;

                // Filter to folders that exist locally
                if !updated.is_empty() {
                    let conn = db_clone.lock().unwrap();
                    let local_folders = db::folders::list_all(&conn).unwrap_or_default();
                    let local_set: std::collections::HashSet<String> =
                        local_folders.into_iter().map(|f| f.id).collect();
                    let ids: Vec<String> = updated
                        .into_iter()
                        .map(|u| u.id)
                        .filter(|id| local_set.contains(id))
                        .collect();
                    Ok(ids)
                } else {
                    Ok(Vec::new())
                }
            })
            .await;

            match result {
                Ok(Ok(ids)) => {
                    if !ids.is_empty() {
                        info!("Remote updates detected for folders: {:?}", ids);
                        let _ = tx.send(SyncTrigger::Folders(ids));
                    }
                }
                Ok(Err(e)) => {
                    warn!("Remote polling error: {}", e);
                }
                Err(e) => {
                    warn!("Remote polling task error: {}", e);
                }
            }

            // Update last check time
            last_check = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S").to_string();
        }
    });
}

/// Sync executor: receives triggers and performs actual sync operations.
fn start_sync_executor(
    app_handle: AppHandle,
    db: Arc<Mutex<Connection>>,
    active_syncs: Arc<Mutex<HashMap<String, RunningSync>>>,
    api_base_url: String,
    app_data_dir: PathBuf,
    mut rx: mpsc::UnboundedReceiver<SyncTrigger>,
    sync_notify: Arc<Notify>,
    tx: mpsc::UnboundedSender<SyncTrigger>,
) {
    tauri::async_runtime::spawn(async move {
        info!("Sync executor started");

        // Trigger initial sync on startup (small delay to let app finish init)
        tokio::time::sleep(Duration::from_secs(2)).await;
        let _ = tx.send(SyncTrigger::All);

        loop {
            tokio::select! {
                Some(trigger) = rx.recv() => {
                    match trigger {
                        SyncTrigger::Folders(ids) => {
                            for folder_id in ids {
                                execute_sync(
                                    &app_handle,
                                    &db,
                                    &active_syncs,
                                    &api_base_url,
                                    &app_data_dir,
                                    &folder_id,
                                ).await;
                            }
                        }
                        SyncTrigger::All => {
                            sync_all_folders(
                                &app_handle,
                                &db,
                                &active_syncs,
                                &api_base_url,
                                &app_data_dir,
                            ).await;
                        }
                    }
                }
                _ = sync_notify.notified() => {
                    // Immediate sync all (from tray menu)
                    info!("Immediate sync triggered (tray/notify)");
                    sync_all_folders(
                        &app_handle,
                        &db,
                        &active_syncs,
                        &api_base_url,
                        &app_data_dir,
                    ).await;
                }
            }
        }
    });
}

async fn sync_all_folders(
    app_handle: &AppHandle,
    db: &Arc<Mutex<Connection>>,
    active_syncs: &Arc<Mutex<HashMap<String, RunningSync>>>,
    api_base_url: &str,
    app_data_dir: &std::path::Path,
) {
    let folders = {
        let conn = db.lock().unwrap();
        match db::folders::list_all(&conn) {
            Ok(f) => f,
            Err(e) => {
                error!("Scheduler: failed to list folders: {}", e);
                return;
            }
        }
    };

    for folder in folders {
        if folder.is_enabled && folder.mode != "cloud_only" {
            execute_sync(app_handle, db, active_syncs, api_base_url, app_data_dir, &folder.id).await;
        }
    }
}

async fn execute_sync(
    app_handle: &AppHandle,
    db: &Arc<Mutex<Connection>>,
    active_syncs: &Arc<Mutex<HashMap<String, RunningSync>>>,
    api_base_url: &str,
    app_data_dir: &std::path::Path,
    folder_id: &str,
) {
    // Dedup: if this folder already has a running sync, skip it.
    {
        let map = active_syncs.lock().unwrap();
        if map.contains_key(folder_id) {
            info!("Scheduler: skipping folder {} (already syncing)", folder_id);
            return;
        }
    }

    // Get folder name for foreground service notification (Android)
    let folder_name = {
        let conn = db.lock().unwrap();
        db::folders::get_by_id(&conn, folder_id)
            .ok()
            .map(|f| f.local_path)
            .unwrap_or_else(|| folder_id.to_string())
    };

    // Start foreground service on Android (no-op on desktop)
    crate::foreground_service::start_foreground(&folder_name);

    let db_clone = db.clone();
    let app_data_dir = app_data_dir.to_path_buf();
    let api_url = api_base_url.to_string();
    let active_syncs_clone = active_syncs.clone();
    let app_handle_clone = app_handle.clone();
    let fid = folder_id.to_string();

    let result = sync::sync_folder_async(
        app_handle_clone,
        active_syncs_clone,
        db_clone,
        app_data_dir,
        api_url,
        fid,
    )
    .await;

    // Stop foreground service on Android (no-op on desktop)
    crate::foreground_service::stop_foreground();

    match result {
        Ok(sync_result) => {
            if sync_result.success {
                info!("Scheduler: synced folder {} successfully", folder_id);
            } else {
                warn!("Scheduler: sync reported failure for folder {} (errors: {:?})",
                    folder_id, sync_result.errors);
            }
        }
        Err(e) => {
            error!("Scheduler: sync error for folder {}: {}", folder_id, e);
        }
    }

    // Emit event so frontend refreshes
    let _ = app_handle.emit("sync-status-changed", folder_id);
}
