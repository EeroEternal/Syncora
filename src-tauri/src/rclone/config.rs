use std::path::PathBuf;
use crate::error::AppError;

/// Generate rclone configuration file content for R2
pub fn generate_config(
    endpoint: &str,
    access_key: &str,
    secret: &str,
) -> String {
    format!(
        "[r2]\ntype = s3\nprovider = Cloudflare\naccess_key_id = {}\nsecret_access_key = {}\nendpoint = {}\nacl = private\nno_check_bucket = true\n",
        access_key, secret, endpoint
    )
}

/// Write rclone config to a file in the app data directory
pub fn write_config(
    app_data_dir: &std::path::Path,
    endpoint: &str,
    access_key: &str,
    secret: &str,
) -> Result<PathBuf, AppError> {
    let config_path = app_data_dir.join("rclone.conf");
    let content = generate_config(endpoint, access_key, secret);
    std::fs::write(&config_path, content)?;
    Ok(config_path)
}
