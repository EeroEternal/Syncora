use std::sync::atomic::Ordering;
use tauri::State;
use crate::auth;
use crate::auth::client::FolderMode;
use crate::db;
use crate::db::folders::Folder;
use crate::error::AppError;
use crate::state::AppState;

/// Sanitize a folder basename into a valid remote prefix.
fn sanitize_remote_prefix(name: &str) -> String {
    let sanitized: String = name
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '.' || c == '-' || c == '_' { c.to_ascii_lowercase() } else { '-' })
        .collect();

    let trimmed = sanitized.trim_matches('-');
    let trimmed = if trimmed.is_empty() { "sync" } else { trimmed };

    let prefix = if trimmed.starts_with(|c: char| c.is_alphanumeric()) {
        trimmed.to_string()
    } else {
        format!("sync-{}", trimmed)
    };

    prefix[..prefix.len().min(128)].to_string()
}

fn path_basename(path: &str) -> String {
    std::path::Path::new(path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("sync")
        .to_string()
}

#[tauri::command]
pub fn list_folders(state: State<AppState>) -> Result<Vec<Folder>, AppError> {
    let conn = state.db.lock().unwrap();
    db::folders::list_all(&conn)
}

#[tauri::command]
pub async fn add_folder(
    state: State<'_, AppState>,
    local_path: String,
    mode: Option<String>,
) -> Result<Folder, AppError> {
    let mode_str = mode.unwrap_or_else(|| "create".into());
    let db = state.db.clone();

    tokio::task::spawn_blocking(move || {
        let folder_mode = match mode_str.as_str() {
            "connect" => FolderMode::Connect,
            "replace" => FolderMode::Replace,
            _ => FolderMode::Create,
        };

        let conn = db.lock().unwrap();
        let settings = db::settings::get(&conn)?;

        // For create mode, check local duplicates
        if folder_mode == FolderMode::Create {
            if db::folders::exists_by_local_path(&conn, &local_path)? {
                return Err(AppError::General("This folder is already being synced".into()));
            }
        }

        // For replace/connect mode, remove local record if exists
        if folder_mode == FolderMode::Replace || folder_mode == FolderMode::Connect {
            let existing = db::folders::list_all(&conn)?
                .into_iter()
                .find(|f| f.local_path == local_path);
            if let Some(f) = existing {
                db::folders::delete(&conn, &f.id)?;
            }
        }

        let basename = path_basename(&local_path);
        let remote_prefix = sanitize_remote_prefix(&basename);

        let tokens = auth::client::ensure_fresh_token(&conn, &settings.api_base_url)?;
        if let Some(t) = &tokens {
            let remote = auth::client::create_folder_remote(
                &settings.api_base_url,
                &t.access_token,
                Some(&local_path),
                &remote_prefix,
                folder_mode,
            ).map_err(|e| {
                let msg = e.to_string();
                if msg.contains("already exists") {
                    AppError::General("A folder with this name is already synced on the server.".into())
                } else {
                    e
                }
            })?;

            db::folders::create(&conn, &remote.id, &local_path, &remote_prefix)
        } else {
            let id = uuid::Uuid::new_v4().to_string();
            db::folders::create(&conn, &id, &local_path, &remote_prefix)
        }
    })
    .await
    .map_err(|e| AppError::General(e.to_string()))?
}

#[tauri::command]
pub async fn delete_folder(
    state: State<'_, AppState>,
    id: String,
) -> Result<(), AppError> {
    // 1. Stop any in-flight sync for this folder and wait for the subprocess to exit.
    let child_to_wait = {
        let mut map = state.active_syncs.lock().unwrap();
        if let Some(rs) = map.remove(&id) {
            rs.cancel_requested.store(true, Ordering::SeqCst);
            let mut child = rs.child.lock().unwrap();
            if let Err(e) = child.kill() {
                if e.kind() != std::io::ErrorKind::InvalidInput {
                    log::warn!("delete_folder: kill failed for {}: {}", id, e);
                }
            }
            Some(rs.child.clone())
        } else {
            None
        }
    };
    if let Some(child) = child_to_wait {
        let mut child = child.lock().unwrap();
        let _ = child.wait();
    }

    // 2. Delete remote record then local DB row.
    let db = state.db.clone();
    let folder_id = id.clone();
    tokio::task::spawn_blocking(move || {
        let conn = db.lock().unwrap();
        let settings = db::settings::get(&conn)?;

        let tokens = auth::client::ensure_fresh_token(&conn, &settings.api_base_url)?;
        if let Some(t) = &tokens {
            let _ = auth::client::delete_folder_remote(
                &settings.api_base_url,
                &t.access_token,
                &folder_id,
            );
        }

        // Best-effort cleanup of local metadata tables (logs/conflicts).
        let _ = conn.execute("DELETE FROM sync_logs WHERE folder_id = ?1", rusqlite::params![&folder_id]);
        let _ = conn.execute("DELETE FROM conflicts WHERE folder_id = ?1", rusqlite::params![&folder_id]);

        db::folders::delete(&conn, &folder_id)
    })
    .await
    .map_err(|e| AppError::General(e.to_string()))?
}
