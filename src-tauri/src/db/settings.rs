use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use crate::error::AppError;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Settings {
    pub r2_endpoint: String,
    pub r2_access_key: String,
    pub r2_secret: String,
    pub r2_bucket: String,
    pub sync_interval_minutes: i64,
    pub auto_start: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            r2_endpoint: String::new(),
            r2_access_key: String::new(),
            r2_secret: String::new(),
            r2_bucket: String::new(),
            sync_interval_minutes: 5,
            auto_start: false,
        }
    }
}

pub fn get(conn: &Connection) -> Result<Settings, AppError> {
    let mut settings = Settings::default();

    let mut stmt = conn.prepare("SELECT key, value FROM settings")?;
    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;

    for row in rows {
        let (key, value) = row?;
        match key.as_str() {
            "r2_endpoint" => settings.r2_endpoint = value,
            "r2_access_key" => settings.r2_access_key = value,
            "r2_secret" => settings.r2_secret = value,
            "r2_bucket" => settings.r2_bucket = value,
            "sync_interval_minutes" => settings.sync_interval_minutes = value.parse().unwrap_or(5),
            "auto_start" => settings.auto_start = value == "true",
            _ => {}
        }
    }

    Ok(settings)
}

pub fn save(conn: &Connection, settings: &Settings) -> Result<(), AppError> {
    let interval_str = settings.sync_interval_minutes.to_string();
    let pairs: Vec<(&str, &str)> = vec![
        ("r2_endpoint", settings.r2_endpoint.as_str()),
        ("r2_access_key", settings.r2_access_key.as_str()),
        ("r2_secret", settings.r2_secret.as_str()),
        ("r2_bucket", settings.r2_bucket.as_str()),
        ("sync_interval_minutes", &interval_str),
        ("auto_start", if settings.auto_start { "true" } else { "false" }),
    ];

    for (key, value) in pairs {
        conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
            params![key, value],
        )?;
    }

    Ok(())
}
