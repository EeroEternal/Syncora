use std::path::Path;
use std::process::Command;
use std::time::Instant;
use crate::error::AppError;
use super::parser::parse_bisync_output;
use super::types::SyncResult;

/// Execute rclone bisync between a local path and an R2 remote path
pub fn run_bisync(
    rclone_path: &Path,
    config_path: &Path,
    local_path: &str,
    bucket: &str,
    remote_prefix: &str,
    needs_resync: bool,
) -> Result<SyncResult, AppError> {
    let remote = format!("r2:{}/{}", bucket, remote_prefix);

    let mut args = vec![
        "bisync".to_string(),
        local_path.to_string(),
        remote,
        "--config".to_string(),
        config_path.to_str().unwrap_or("").to_string(),
        "--checksum".to_string(),
        "--create-empty-src-dirs".to_string(),
        "--resilient".to_string(),
        "--verbose".to_string(),
    ];

    if needs_resync {
        args.push("--resync".to_string());
    }

    let start = Instant::now();

    let output = Command::new(rclone_path)
        .args(&args)
        .output()
        .map_err(|e| AppError::Rclone(format!("Failed to execute rclone: {}", e)))?;

    let duration_ms = start.elapsed().as_millis() as u64;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let exit_code = output.status.code().unwrap_or(-1);

    log::info!("rclone bisync completed in {}ms (exit code: {})", duration_ms, exit_code);
    if !stderr.is_empty() {
        log::debug!("rclone stderr: {}", stderr);
    }

    Ok(parse_bisync_output(&stdout, &stderr, exit_code, duration_ms))
}

/// Test if rclone can connect to the R2 remote
pub fn test_connection(
    rclone_path: &Path,
    config_path: &Path,
    bucket: &str,
) -> Result<(bool, String), AppError> {
    let output = Command::new(rclone_path)
        .args([
            "lsd",
            &format!("r2:{}", bucket),
            "--config",
            config_path.to_str().unwrap_or(""),
            "--max-depth",
            "1",
        ])
        .output()
        .map_err(|e| AppError::Rclone(format!("Failed to execute rclone: {}", e)))?;

    if output.status.success() {
        Ok((true, "Connection successful".to_string()))
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        Ok((false, format!("Connection failed: {}", stderr.lines().next().unwrap_or("Unknown error"))))
    }
}
