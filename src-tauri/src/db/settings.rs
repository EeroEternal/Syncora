use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use crate::error::AppError;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Settings {
    pub api_base_url: String,
    pub sync_interval_minutes: i64,
    pub auto_start: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            api_base_url: "https://api.synchora.cc".to_string(),
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
            "api_base_url" => settings.api_base_url = value,
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
        ("api_base_url", settings.api_base_url.as_str()),
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
