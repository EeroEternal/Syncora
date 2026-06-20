use reqwest::blocking::Client;
use serde::{Deserialize, Deserializer, Serialize};
use crate::error::AppError;
use crate::auth::token_store::{self, AuthTokens};
use rusqlite::Connection;

const USER_AGENT: &str = "Syncora/0.1.0";

/// D1 stores booleans as integers (0/1). This deserializer handles both.
fn deserialize_bool_from_int<'de, D: Deserializer<'de>>(deserializer: D) -> Result<bool, D::Error> {
    let v: serde_json::Value = Deserialize::deserialize(deserializer)?;
    match v {
        serde_json::Value::Bool(b) => Ok(b),
        serde_json::Value::Number(n) => Ok(n.as_i64().unwrap_or(0) != 0),
        _ => Ok(false),
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct UserProfile {
    pub id: String,
    pub email: String,
    pub display_name: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AuthResponse {
    pub user: UserProfile,
    pub access_token: String,
    pub refresh_token: String,
}

#[derive(Debug, Deserialize)]
pub struct TokenPair {
    pub access_token: String,
    pub refresh_token: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SyncCredentials {
    pub access_key_id: String,
    pub secret_access_key: String,
    pub endpoint: String,
    pub bucket: String,
    pub remote_path: String,
    #[serde(deserialize_with = "deserialize_bool_from_int")]
    pub needs_resync: bool,
}

#[derive(Debug, Serialize)]
pub struct SyncReport {
    pub folder_id: String,
    pub success: bool,
    pub files_transferred: u64,
    pub files_deleted: u64,
    pub duration_ms: u64,
    pub errors: Vec<String>,
    pub conflicts: Vec<ConflictReport>,
}

#[derive(Debug, Serialize)]
pub struct ConflictReport {
    pub file_path: String,
    pub local_version: Option<String>,
    pub remote_version: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RemoteFolder {
    pub id: String,
    pub user_id: String,
    pub local_path_hint: Option<String>,
    pub remote_prefix: String,
    pub mode: String,
    pub last_sync_at: Option<String>,
    pub status: String,
    #[serde(deserialize_with = "deserialize_bool_from_int")]
    pub is_enabled: bool,
    #[serde(deserialize_with = "deserialize_bool_from_int")]
    pub needs_resync: bool,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct RemoteConflict {
    pub id: String,
    pub folder_id: String,
    pub file_path: String,
    pub local_version: Option<String>,
    pub remote_version: Option<String>,
    pub detected_at: String,
    pub resolved: bool,
    pub resolution: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RemoteSyncLog {
    pub id: String,
    pub folder_id: String,
    pub timestamp: String,
    pub action: String,
    pub status: String,
    pub message: Option<String>,
    pub duration_ms: Option<i64>,
}

fn http_client() -> Client {
    Client::builder()
        .user_agent(USER_AGENT)
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .expect("Failed to build HTTP client")
}

fn auth_header(token: &str) -> String {
    format!("Bearer {}", token)
}

// ---- Auth API ----

pub fn register(
    base_url: &str,
    email: &str,
    password: &str,
) -> Result<AuthResponse, AppError> {
    let client = http_client();
    let resp = client
        .post(format!("{}/api/v1/auth/register", base_url))
        .json(&serde_json::json!({
            "email": email,
            "password": password,
        }))
        .send()
        .map_err(|e| AppError::Api(e.to_string()))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().unwrap_or_default();
        return Err(AppError::Api(format!("Register failed ({}): {}", status, body)));
    }

    resp.json::<AuthResponse>()
        .map_err(|e| AppError::Api(format!("Failed to parse auth response: {}", e)))
}

pub fn login(
    base_url: &str,
    email: &str,
    password: &str,
) -> Result<AuthResponse, AppError> {
    let client = http_client();
    let resp = client
        .post(format!("{}/api/v1/auth/login", base_url))
        .json(&serde_json::json!({
            "email": email,
            "password": password,
        }))
        .send()
        .map_err(|e| AppError::Api(e.to_string()))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().unwrap_or_default();
        return Err(AppError::Api(format!("Login failed ({}): {}", status, body)));
    }

    resp.json::<AuthResponse>()
        .map_err(|e| AppError::Api(format!("Failed to parse auth response: {}", e)))
}

pub fn refresh_token(
    base_url: &str,
    refresh_token: &str,
) -> Result<TokenPair, AppError> {
    let client = http_client();
    let resp = client
        .post(format!("{}/api/v1/auth/refresh", base_url))
        .json(&serde_json::json!({
            "refresh_token": refresh_token,
        }))
        .send()
        .map_err(|e| AppError::Api(e.to_string()))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().unwrap_or_default();
        return Err(AppError::Api(format!("Token refresh failed ({}): {}", status, body)));
    }

    resp.json::<TokenPair>()
        .map_err(|e| AppError::Api(format!("Failed to parse token response: {}", e)))
}

pub fn logout(
    base_url: &str,
    access_token: &str,
    refresh_token: &str,
) -> Result<(), AppError> {
    let client = http_client();
    let _ = client
        .post(format!("{}/api/v1/auth/logout", base_url))
        .header("Authorization", auth_header(access_token))
        .header("X-Refresh-Token", refresh_token)
        .send();
    Ok(())
}

pub fn get_me(
    base_url: &str,
    access_token: &str,
) -> Result<UserProfile, AppError> {
    let client = http_client();
    let resp = client
        .get(format!("{}/api/v1/auth/me", base_url))
        .header("Authorization", auth_header(access_token))
        .send()
        .map_err(|e| AppError::Api(e.to_string()))?;

    if !resp.status().is_success() {
        return Err(AppError::Api("Failed to get user profile".into()));
    }

    let wrapper: serde_json::Value = resp.json().map_err(|e| AppError::Api(e.to_string()))?;
    serde_json::from_value(wrapper["user"].clone())
        .map_err(|e| AppError::Api(format!("Failed to parse user: {}", e)))
}

// ---- Sync API ----

pub fn get_sync_credentials(
    base_url: &str,
    access_token: &str,
    folder_id: &str,
) -> Result<SyncCredentials, AppError> {
    let client = http_client();
    let resp = client
        .post(format!("{}/api/v1/sync/credentials", base_url))
        .header("Authorization", auth_header(access_token))
        .json(&serde_json::json!({ "folder_id": folder_id }))
        .send()
        .map_err(|e| AppError::Api(e.to_string()))?;

    if !resp.status().is_success() {
        let body = resp.text().unwrap_or_default();
        return Err(AppError::Api(format!("Failed to get sync credentials: {}", body)));
    }

    resp.json::<SyncCredentials>()
        .map_err(|e| AppError::Api(format!("Failed to parse credentials: {}", e)))
}

pub fn report_sync(
    base_url: &str,
    access_token: &str,
    report: &SyncReport,
) -> Result<(), AppError> {
    let client = http_client();
    let resp = client
        .post(format!("{}/api/v1/sync/report", base_url))
        .header("Authorization", auth_header(access_token))
        .json(report)
        .send()
        .map_err(|e| AppError::Api(e.to_string()))?;

    if !resp.status().is_success() {
        log::warn!("Failed to report sync result: {}", resp.status());
    }

    Ok(())
}

/// Response from GET /sync/updates
#[derive(Debug, Deserialize)]
pub struct RemoteUpdatesResponse {
    pub updated_folders: Vec<UpdatedFolder>,
}

#[derive(Debug, Deserialize)]
pub struct UpdatedFolder {
    pub id: String,
    pub updated_at: String,
}

/// Check which folders have been updated on the server since `since` timestamp.
pub fn check_remote_updates(
    base_url: &str,
    access_token: &str,
    since: &str,
) -> Result<Vec<UpdatedFolder>, AppError> {
    let client = http_client();
    let resp = client
        .get(format!("{}/api/v1/sync/updates?since={}", base_url, since))
        .header("Authorization", auth_header(access_token))
        .send()
        .map_err(|e| AppError::Api(e.to_string()))?;

    if !resp.status().is_success() {
        let body = resp.text().unwrap_or_default();
        return Err(AppError::Api(format!("Failed to check remote updates: {}", body)));
    }

    let data: RemoteUpdatesResponse = resp.json()
        .map_err(|e| AppError::Api(format!("Failed to parse updates response: {}", e)))?;

    Ok(data.updated_folders)
}

// ---- Folder API ----

/// Mode for creating/connecting a folder on the backend.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FolderMode {
    /// Create a new folder; error if one with the same prefix exists.
    Create,
    /// Connect to an existing folder by prefix (update local_path_hint).
    Connect,
    /// Replace: delete existing folder data and re-create.
    Replace,
}

pub fn create_folder_remote(
    base_url: &str,
    access_token: &str,
    local_path_hint: Option<&str>,
    remote_prefix: &str,
    mode: FolderMode,
) -> Result<RemoteFolder, AppError> {
    let client = http_client();
    let mut body = serde_json::json!({ "remote_prefix": remote_prefix });
    if let Some(hint) = local_path_hint {
        body["local_path_hint"] = serde_json::Value::String(hint.to_string());
    }
    match mode {
        FolderMode::Replace => { body["force"] = serde_json::Value::Bool(true); }
        FolderMode::Connect => { body["connect"] = serde_json::Value::Bool(true); }
        FolderMode::Create => {}
    }

    let resp = client
        .post(format!("{}/api/v1/folders", base_url))
        .header("Authorization", auth_header(access_token))
        .json(&body)
        .send()
        .map_err(|e| AppError::Api(e.to_string()))?;

    if !resp.status().is_success() {
        let body = resp.text().unwrap_or_default();
        return Err(AppError::Api(format!("Failed to create folder: {}", body)));
    }

    let wrapper: serde_json::Value = resp.json().map_err(|e| AppError::Api(e.to_string()))?;
    serde_json::from_value(wrapper["folder"].clone())
        .map_err(|e| AppError::Api(format!("Failed to parse folder: {}", e)))
}

pub fn delete_folder_remote(
    base_url: &str,
    access_token: &str,
    folder_id: &str,
) -> Result<(), AppError> {
    let client = http_client();
    let resp = client
        .delete(format!("{}/api/v1/folders/{}", base_url, folder_id))
        .header("Authorization", auth_header(access_token))
        .send()
        .map_err(|e| AppError::Api(e.to_string()))?;

    if !resp.status().is_success() {
        let body = resp.text().unwrap_or_default();
        return Err(AppError::Api(format!("Failed to delete folder: {}", body)));
    }

    Ok(())
}

pub fn resolve_conflict_remote(
    base_url: &str,
    access_token: &str,
    conflict_id: &str,
    resolution: &str,
) -> Result<(), AppError> {
    let client = http_client();
    let resp = client
        .patch(format!("{}/api/v1/conflicts/{}", base_url, conflict_id))
        .header("Authorization", auth_header(access_token))
        .json(&serde_json::json!({ "resolution": resolution }))
        .send()
        .map_err(|e| AppError::Api(e.to_string()))?;

    if !resp.status().is_success() {
        let body = resp.text().unwrap_or_default();
        return Err(AppError::Api(format!("Failed to resolve conflict: {}", body)));
    }

    Ok(())
}

// ---- Token auto-refresh ----

/// Check if a JWT is expired or about to expire (within 60s buffer).
fn is_jwt_expired(access_token: &str) -> bool {
    let parts: Vec<&str> = access_token.split('.').collect();
    if parts.len() != 3 {
        return true;
    }

    let payload = parts[1];
    let decoded = match base64_decode_url(payload) {
        Some(d) => d,
        None => return true,
    };

    let json: serde_json::Value = match serde_json::from_slice(&decoded) {
        Ok(v) => v,
        Err(_) => return true,
    };

    let exp = match json.get("exp").and_then(|v| v.as_u64()) {
        Some(e) => e,
        None => return true,
    };

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    now + 60 >= exp
}

fn base64_decode_url(input: &str) -> Option<Vec<u8>> {
    let padded = match input.len() % 4 {
        2 => format!("{}==", input),
        3 => format!("{}=", input),
        _ => input.to_string(),
    };
    let url_safe = padded.replace('-', "+").replace('_', "/");
    use std::io::Read;
    let mut decoder = B64Reader::new(url_safe.as_bytes());
    let mut buf = Vec::new();
    decoder.read_to_end(&mut buf).ok()?;
    Some(buf)
}

struct B64Reader<'a> {
    input: &'a [u8],
    pos: usize,
}

impl<'a> B64Reader<'a> {
    fn new(input: &'a [u8]) -> Self {
        B64Reader { input, pos: 0 }
    }
}

impl<'a> std::io::Read for B64Reader<'a> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        const TABLE: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut written = 0;
        let mut accum: u32 = 0;
        let mut bits: u32 = 0;

        while self.pos < self.input.len() && written < buf.len() {
            let ch = self.input[self.pos];
            self.pos += 1;
            if ch == b'=' { break; }
            let val = match TABLE.iter().position(|&c| c == ch) {
                Some(v) => v as u32,
                None => continue,
            };
            accum = (accum << 6) | val;
            bits += 6;
            if bits >= 8 {
                bits -= 8;
                buf[written] = ((accum >> bits) & 0xFF) as u8;
                written += 1;
            }
        }
        Ok(written)
    }
}

/// Load tokens and auto-refresh if expired. Returns fresh tokens.
pub fn ensure_fresh_token(
    conn: &Connection,
    base_url: &str,
) -> Result<Option<AuthTokens>, AppError> {
    let tokens = match token_store::load_tokens(conn)? {
        Some(t) => t,
        None => return Ok(None),
    };

    if !is_jwt_expired(&tokens.access_token) {
        return Ok(Some(tokens));
    }

    log::debug!("Access token expired, refreshing...");

    let pair = match refresh_token(base_url, &tokens.refresh_token) {
        Ok(p) => p,
        Err(AppError::Api(ref msg)) if msg.contains("401") || msg.contains("Invalid or expired") => {
            // Refresh token is genuinely invalid/revoked — user must sign in again.
            log::warn!("Refresh token rejected by server (401), clearing session");
            return Err(AppError::Auth("Session expired, please sign in again".into()));
        }
        Err(e) => {
            // Network error or transient failure — keep existing tokens, let the
            // caller retry later. Don't kick the user out for connectivity issues.
            log::warn!("Token refresh failed (network/transient), reusing existing token: {}", e);
            return Ok(Some(tokens));
        }
    };

    token_store::update_tokens(conn, &pair.access_token, &pair.refresh_token)?;

    Ok(Some(AuthTokens {
        access_token: pair.access_token,
        refresh_token: pair.refresh_token,
        ..tokens
    }))
}
