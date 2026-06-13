pub mod migrations;
pub mod folders;
pub mod conflicts;
pub mod sync_logs;
pub mod settings;

use rusqlite::Connection;
use crate::error::AppError;

pub fn init_database(app_data_dir: &std::path::Path) -> Result<Connection, AppError> {
    std::fs::create_dir_all(app_data_dir)?;
    let db_path = app_data_dir.join("syncora.db");
    let conn = Connection::open(db_path)?;

    // Enable WAL mode for better performance
    conn.execute_batch("PRAGMA journal_mode=WAL;")?;

    // Run migrations
    migrations::run(&conn)?;

    Ok(conn)
}
