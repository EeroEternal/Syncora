use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use crate::error::AppError;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SyncLog {
    pub id: String,
    pub folder_id: String,
    pub timestamp: String,
    pub action: String,
    pub status: String,
    pub message: Option<String>,
    pub duration_ms: Option<i64>,
}

pub fn insert(
    conn: &Connection,
    id: &str,
    folder_id: &str,
    action: &str,
    status: &str,
    message: Option<&str>,
    duration_ms: Option<i64>,
) -> Result<(), AppError> {
    conn.execute(
        "INSERT INTO sync_logs (id, folder_id, action, status, message, duration_ms) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![id, folder_id, action, status, message, duration_ms],
    )?;
    Ok(())
}

pub fn get_logs(conn: &Connection, folder_id: Option<&str>, limit: i64) -> Result<Vec<SyncLog>, AppError> {
    let (query, params_vec): (String, Vec<Box<dyn rusqlite::types::ToSql>>) = match folder_id {
        Some(fid) => (
            "SELECT id, folder_id, timestamp, action, status, message, duration_ms FROM sync_logs WHERE folder_id = ?1 ORDER BY timestamp DESC LIMIT ?2".to_string(),
            vec![Box::new(fid.to_string()), Box::new(limit)],
        ),
        None => (
            "SELECT id, folder_id, timestamp, action, status, message, duration_ms FROM sync_logs ORDER BY timestamp DESC LIMIT ?1".to_string(),
            vec![Box::new(limit)],
        ),
    };

    let mut stmt = conn.prepare(&query)?;
    let params_refs: Vec<&dyn rusqlite::types::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();
    let logs = stmt.query_map(params_refs.as_slice(), |row| {
        Ok(SyncLog {
            id: row.get(0)?,
            folder_id: row.get(1)?,
            timestamp: row.get(2)?,
            action: row.get(3)?,
            status: row.get(4)?,
            message: row.get(5)?,
            duration_ms: row.get(6)?,
        })
    })?.collect::<Result<Vec<_>, _>>()?;

    Ok(logs)
}
