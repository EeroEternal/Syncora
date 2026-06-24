use std::collections::HashMap;
use rusqlite::{params, Connection};
use crate::error::AppError;

use crate::s3sync::types::FileEntry;

/// Load the last-sync state for a folder.
/// Returns a map of relative_path → FileEntry.
pub fn load_state(conn: &Connection, folder_id: &str) -> Result<HashMap<String, FileEntry>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT file_path, etag, size, local_mtime FROM sync_state WHERE folder_id = ?1"
    )?;
    let entries = stmt.query_map(params![folder_id], |row| {
        let path: String = row.get(0)?;
        let etag: Option<String> = row.get(1)?;
        let size: i64 = row.get(2)?;
        let mtime_secs: Option<i64> = row.get(3)?;
        let mtime = mtime_secs
            .and_then(|s| chrono::DateTime::from_timestamp(s, 0));
        Ok((
            path,
            FileEntry {
                path: String::new(), // not needed for state comparison
                size: size as u64,
                etag,
                mtime,
            },
        ))
    })?;
    let mut map = HashMap::new();
    for entry in entries {
        let (path, file_entry) = entry?;
        map.insert(path, file_entry);
    }
    Ok(map)
}

/// Save the current state after a successful sync.
/// Replaces all state for the given folder.
pub fn save_state(
    conn: &Connection,
    folder_id: &str,
    entries: &HashMap<String, FileEntry>,
) -> Result<(), AppError> {
    // Clear old state
    conn.execute(
        "DELETE FROM sync_state WHERE folder_id = ?1",
        params![folder_id],
    )?;

    // Insert new state
    for (path, entry) in entries {
        let mtime_secs = entry.mtime.map(|t| t.timestamp());
        conn.execute(
            "INSERT OR REPLACE INTO sync_state (folder_id, file_path, etag, size, local_mtime) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                folder_id,
                path,
                entry.etag,
                entry.size as i64,
                mtime_secs,
            ],
        )?;
    }
    Ok(())
}

/// Clear all sync state for a folder.
pub fn clear_state(conn: &Connection, folder_id: &str) -> Result<(), AppError> {
    conn.execute(
        "DELETE FROM sync_state WHERE folder_id = ?1",
        params![folder_id],
    )?;
    Ok(())
}
