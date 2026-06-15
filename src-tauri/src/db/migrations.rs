use rusqlite::Connection;
use crate::error::AppError;

pub fn run(conn: &Connection) -> Result<(), AppError> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS folders (
            id TEXT PRIMARY KEY,
            local_path TEXT NOT NULL UNIQUE,
            remote_prefix TEXT NOT NULL,
            mode TEXT NOT NULL DEFAULT 'normal' CHECK(mode IN ('normal','cloud_only')),
            last_sync_at TEXT,
            status TEXT NOT NULL DEFAULT 'idle' CHECK(status IN ('idle','syncing','synced','error','released')),
            is_enabled INTEGER NOT NULL DEFAULT 1,
            needs_resync INTEGER NOT NULL DEFAULT 1,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS conflicts (
            id TEXT PRIMARY KEY,
            folder_id TEXT NOT NULL REFERENCES folders(id) ON DELETE CASCADE,
            file_path TEXT NOT NULL,
            local_version TEXT,
            remote_version TEXT,
            detected_at TEXT NOT NULL DEFAULT (datetime('now')),
            resolved INTEGER NOT NULL DEFAULT 0,
            resolution TEXT CHECK(resolution IN ('keep_local','keep_remote','keep_both'))
        );

        CREATE TABLE IF NOT EXISTS sync_logs (
            id TEXT PRIMARY KEY,
            folder_id TEXT NOT NULL REFERENCES folders(id) ON DELETE CASCADE,
            timestamp TEXT NOT NULL DEFAULT (datetime('now')),
            action TEXT NOT NULL,
            status TEXT NOT NULL CHECK(status IN ('success','error','warning')),
            message TEXT,
            duration_ms INTEGER
        );

        CREATE TABLE IF NOT EXISTS settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS auth (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );
        "
    )?;
    Ok(())
}
