use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use crate::error::AppError;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Conflict {
    pub id: String,
    pub folder_id: String,
    pub file_path: String,
    pub local_version: Option<String>,
    pub remote_version: Option<String>,
    pub detected_at: String,
    pub resolved: bool,
    pub resolution: Option<String>,
}

pub fn list(conn: &Connection, resolved: Option<bool>) -> Result<Vec<Conflict>, AppError> {
    let query = match resolved {
        Some(true) => "SELECT id, folder_id, file_path, local_version, remote_version, detected_at, resolved, resolution FROM conflicts WHERE resolved = 1 ORDER BY detected_at DESC",
        Some(false) => "SELECT id, folder_id, file_path, local_version, remote_version, detected_at, resolved, resolution FROM conflicts WHERE resolved = 0 ORDER BY detected_at DESC",
        None => "SELECT id, folder_id, file_path, local_version, remote_version, detected_at, resolved, resolution FROM conflicts ORDER BY detected_at DESC",
    };

    let mut stmt = conn.prepare(query)?;
    let conflicts = stmt.query_map([], |row| {
        Ok(Conflict {
            id: row.get(0)?,
            folder_id: row.get(1)?,
            file_path: row.get(2)?,
            local_version: row.get(3)?,
            remote_version: row.get(4)?,
            detected_at: row.get(5)?,
            resolved: row.get(6)?,
            resolution: row.get(7)?,
        })
    })?.collect::<Result<Vec<_>, _>>()?;

    Ok(conflicts)
}

pub fn create(
    conn: &Connection,
    id: &str,
    folder_id: &str,
    file_path: &str,
    local_version: Option<&str>,
    remote_version: Option<&str>,
) -> Result<(), AppError> {
    conn.execute(
        "INSERT INTO conflicts (id, folder_id, file_path, local_version, remote_version) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![id, folder_id, file_path, local_version, remote_version],
    )?;
    Ok(())
}

pub fn resolve(conn: &Connection, id: &str, resolution: &str) -> Result<(), AppError> {
    conn.execute(
        "UPDATE conflicts SET resolved = 1, resolution = ?1 WHERE id = ?2",
        params![resolution, id],
    )?;
    Ok(())
}

pub fn count_unresolved(conn: &Connection) -> Result<i64, AppError> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM conflicts WHERE resolved = 0",
        [],
        |row| row.get(0),
    )?;
    Ok(count)
}
