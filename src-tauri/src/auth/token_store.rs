use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use crate::error::AppError;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AuthTokens {
    pub access_token: String,
    pub refresh_token: String,
    pub user_id: String,
    pub email: String,
    pub display_name: Option<String>,
}

pub fn save_tokens(conn: &Connection, tokens: &AuthTokens) -> Result<(), AppError> {
    let pairs: Vec<(&str, &str)> = vec![
        ("access_token", &tokens.access_token),
        ("refresh_token", &tokens.refresh_token),
        ("user_id", &tokens.user_id),
        ("email", &tokens.email),
        ("display_name", tokens.display_name.as_deref().unwrap_or("")),
    ];

    for (key, value) in pairs {
        conn.execute(
            "INSERT OR REPLACE INTO auth (key, value) VALUES (?1, ?2)",
            params![key, value],
        )?;
    }

    Ok(())
}

pub fn load_tokens(conn: &Connection) -> Result<Option<AuthTokens>, AppError> {
    let mut stmt = conn.prepare("SELECT key, value FROM auth")?;
    let mut map = std::collections::HashMap::new();

    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;

    for row in rows {
        let (key, value) = row?;
        map.insert(key, value);
    }

    let access_token = match map.get("access_token") {
        Some(t) if !t.is_empty() => t.clone(),
        _ => return Ok(None),
    };

    let refresh_token = match map.get("refresh_token") {
        Some(t) if !t.is_empty() => t.clone(),
        _ => return Ok(None),
    };

    Ok(Some(AuthTokens {
        access_token,
        refresh_token,
        user_id: map.get("user_id").cloned().unwrap_or_default(),
        email: map.get("email").cloned().unwrap_or_default(),
        display_name: map
            .get("display_name")
            .filter(|s| !s.is_empty())
            .cloned(),
    }))
}

pub fn clear_tokens(conn: &Connection) -> Result<(), AppError> {
    conn.execute("DELETE FROM auth", [])?;
    Ok(())
}

pub fn update_access_token(conn: &Connection, access_token: &str) -> Result<(), AppError> {
    conn.execute(
        "INSERT OR REPLACE INTO auth (key, value) VALUES ('access_token', ?1)",
        params![access_token],
    )?;
    Ok(())
}

pub fn update_tokens(conn: &Connection, access_token: &str, refresh_token: &str) -> Result<(), AppError> {
    conn.execute(
        "INSERT OR REPLACE INTO auth (key, value) VALUES ('access_token', ?1)",
        params![access_token],
    )?;
    conn.execute(
        "INSERT OR REPLACE INTO auth (key, value) VALUES ('refresh_token', ?1)",
        params![refresh_token],
    )?;
    Ok(())
}
