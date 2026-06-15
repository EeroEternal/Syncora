use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use crate::error::AppError;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Folder {
    pub id: String,
    pub local_path: String,
    pub remote_prefix: String,
    pub mode: String,
    pub last_sync_at: Option<String>,
    pub status: String,
    pub is_enabled: bool,
    pub needs_resync: bool,
    pub created_at: String,
}

pub fn list_all(conn: &Connection) -> Result<Vec<Folder>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT id, local_path, remote_prefix, mode, last_sync_at, status, is_enabled, needs_resync, created_at FROM folders ORDER BY created_at DESC"
    )?;

    let folders = stmt.query_map([], |row| {
        Ok(Folder {
            id: row.get(0)?,
            local_path: row.get(1)?,
            remote_prefix: row.get(2)?,
            mode: row.get(3)?,
            last_sync_at: row.get(4)?,
            status: row.get(5)?,
            is_enabled: row.get(6)?,
            needs_resync: row.get(7)?,
            created_at: row.get(8)?,
        })
    })?.collect::<Result<Vec<_>, _>>()?;

    Ok(folders)
}

pub fn get_by_id(conn: &Connection, id: &str) -> Result<Folder, AppError> {
    conn.query_row(
        "SELECT id, local_path, remote_prefix, mode, last_sync_at, status, is_enabled, needs_resync, created_at FROM folders WHERE id = ?1",
        params![id],
        |row| {
            Ok(Folder {
                id: row.get(0)?,
                local_path: row.get(1)?,
                remote_prefix: row.get(2)?,
                mode: row.get(3)?,
                last_sync_at: row.get(4)?,
                status: row.get(5)?,
                is_enabled: row.get(6)?,
                needs_resync: row.get(7)?,
                created_at: row.get(8)?,
            })
        },
    ).map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => AppError::NotFound(format!("Folder not found: {}", id)),
        _ => AppError::Database(e),
    })
}

pub fn create(conn: &Connection, id: &str, local_path: &str, remote_prefix: &str) -> Result<Folder, AppError> {
    conn.execute(
        "INSERT INTO folders (id, local_path, remote_prefix) VALUES (?1, ?2, ?3)",
        params![id, local_path, remote_prefix],
    )?;
    get_by_id(conn, id)
}

pub fn update_status(conn: &Connection, id: &str, status: &str) -> Result<(), AppError> {
    conn.execute(
        "UPDATE folders SET status = ?1 WHERE id = ?2",
        params![status, id],
    )?;
    Ok(())
}

pub fn update_last_sync(conn: &Connection, id: &str) -> Result<(), AppError> {
    conn.execute(
        "UPDATE folders SET last_sync_at = datetime('now'), status = 'synced', needs_resync = 0 WHERE id = ?1",
        params![id],
    )?;
    Ok(())
}

pub fn update_mode(conn: &Connection, id: &str, mode: &str) -> Result<(), AppError> {
    conn.execute(
        "UPDATE folders SET mode = ?1, status = CASE WHEN ?1 = 'cloud_only' THEN 'released' ELSE status END WHERE id = ?2",
        params![mode, id],
    )?;
    Ok(())
}

pub fn delete(conn: &Connection, id: &str) -> Result<(), AppError> {
    conn.execute("DELETE FROM folders WHERE id = ?1", params![id])?;
    Ok(())
}

/// Check if a folder with the given local_path already exists.
pub fn exists_by_local_path(conn: &Connection, local_path: &str) -> Result<bool, AppError> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM folders WHERE local_path = ?1",
        params![local_path],
        |row| row.get(0),
    )?;
    Ok(count > 0)
}
