use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Instant;

use tauri::{AppHandle, Emitter};

use crate::error::AppError;
use crate::state::{RunningSync};
use super::parser::parse_bisync_output;
use super::types::SyncResult;

/// Execute rclone bisync between a local path and an R2 remote path.
///
/// Behavior:
/// 1. If `needs_resync` is true OR rclone bisync state files don't exist locally,
///    run with `--resync` (treats local as source of truth, uploads everything).
/// 2. Otherwise run normal incremental bisync with `--resilient`.
/// 3. If a normal bisync still hits "missing state files" (e.g. cache was wiped
///    between our check and rclone's read), auto-retry with `--resync`.
///
/// The subprocess is registered in `active_syncs` under `folder_id`, so the
/// user can cancel it via the `cancel_sync` command. `cancel_requested` is an
/// `Arc<AtomicBool>` polled periodically; when set to true the child is killed.
/// Progress events are emitted to the frontend via `sync-progress`.
pub fn run_bisync(
    app_handle: &AppHandle,
    active_syncs: &Arc<Mutex<HashMap<String, RunningSync>>>,
    folder_id: &str,
    rclone_path: &Path,
    config_path: &Path,
    local_path: &str,
    bucket: &str,
    remote_prefix: &str,
    needs_resync: bool,
) -> Result<SyncResult, AppError> {
    let state_files_exist = bisync_state_files_exist(local_path, bucket, remote_prefix);
    let must_resync = needs_resync || !state_files_exist;

    if must_resync && !needs_resync {
        log::info!(
            "No bisync state files found for {} – running --resync (first-time init)",
            local_path
        );
    }

    // If cancel was requested before we even start, bail out immediately.
    if cancel_requested(active_syncs, folder_id) {
        return Ok(cancelled_result(0));
    }

    let result = run_bisync_inner(
        app_handle,
        active_syncs,
        folder_id,
        rclone_path,
        config_path,
        local_path,
        bucket,
        remote_prefix,
        must_resync,
    )?;

    // Defensive auto-retry: if a non-resync run still complains about missing
    // state files, retry with --resync once.
    if !result.success && !must_resync && is_missing_state_files_error(&result) {
        if cancel_requested(active_syncs, folder_id) {
            return Ok(cancelled_result(result.duration_ms));
        }
        log::info!("Bisync state files missing mid-run – retrying with --resync (local wins)");
        let retry = run_bisync_inner(
            app_handle,
            active_syncs,
            folder_id,
            rclone_path,
            config_path,
            local_path,
            bucket,
            remote_prefix,
            true,
        )?;
        if !retry.success {
            log::warn!(
                "--resync retry failed for {} (errors: {:?})",
                local_path, retry.errors
            );
        }
        return Ok(retry);
    }

    Ok(result)
}

fn is_missing_state_files_error(result: &SyncResult) -> bool {
    result.errors.iter().any(|e| {
        e.contains("cannot find prior Path1 or Path2 listings")
    })
}

fn cancel_requested(
    active_syncs: &Arc<Mutex<HashMap<String, RunningSync>>>,
    folder_id: &str,
) -> bool {
    active_syncs
        .lock()
        .unwrap()
        .get(folder_id)
        .map(|rs| rs.cancel_requested.load(Ordering::SeqCst))
        .unwrap_or(false)
}

/// Helper to build a "cancelled" SyncResult.
fn cancelled_result(duration_ms: u64) -> SyncResult {
    SyncResult {
        success: false,
        files_transferred: 0,
        files_deleted: 0,
        transferred_paths: Vec::new(),
        errors: vec!["Sync cancelled by user".to_string()],
        conflicts: Vec::new(),
        duration_ms,
    }
}

/// Check whether rclone has previously written bisync state files for this pair.
fn bisync_state_files_exist(local_path: &str, bucket: &str, remote_prefix: &str) -> bool {
    let Some(cache_dir) = bisync_cache_dir() else {
        return false;
    };
    let base_name = encode_bisync_pair(local_path, bucket, remote_prefix);
    let path1 = cache_dir.join(format!("{}.path1.lst", base_name));
    let path2 = cache_dir.join(format!("{}.path2.lst", base_name));
    path1.exists() && path2.exists()
}

fn bisync_cache_dir() -> Option<PathBuf> {
    if let Ok(custom) = std::env::var("RCLONE_CACHE_DIR") {
        return Some(PathBuf::from(custom).join("bisync"));
    }
    #[cfg(target_os = "macos")]
    {
        let home = std::env::var("HOME").ok()?;
        Some(PathBuf::from(home).join("Library/Caches/rclone/bisync"))
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        if let Ok(xdg) = std::env::var("XDG_CACHE_HOME") {
            return Some(PathBuf::from(xdg).join("rclone/bisync"));
        }
        let home = std::env::var("HOME").ok()?;
        Some(PathBuf::from(home).join(".cache/rclone/bisync"))
    }
    #[cfg(target_os = "windows")]
    {
        let appdata = std::env::var("LOCALAPPDATA").ok()?;
        Some(PathBuf::from(appdata).join("rclone/bisync"))
    }
}

/// Replicate rclone's bisync state-file naming: replace `:` and `/` with `_`,
/// strip leading `_`, and join the two endpoints with `..`.
fn encode_bisync_pair(local_path: &str, bucket: &str, remote_prefix: &str) -> String {
    let p1 = encode_path_component(local_path);
    let p2 = encode_path_component(&format!("r2:{}/{}", bucket, remote_prefix));
    format!("{}..{}", p1, p2)
}

fn encode_path_component(s: &str) -> String {
    let replaced: String = s
        .chars()
        .map(|c| if c == '/' || c == ':' { '_' } else { c })
        .collect();
    replaced.trim_start_matches('_').to_string()
}

fn run_bisync_inner(
    app_handle: &AppHandle,
    active_syncs: &Arc<Mutex<HashMap<String, RunningSync>>>,
    folder_id: &str,
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
        "--verbose".to_string(),
    ];

    if needs_resync {
        args.push("--resync".to_string());
        log::info!("Running bisync --resync for {} -> r2:{}/{}", local_path, bucket, remote_prefix);
    } else {
        args.push("--resilient".to_string());
        log::info!("Running bisync for {} -> r2:{}/{}", local_path, bucket, remote_prefix);
    }

    let start = Instant::now();

    // Spawn the subprocess (we keep the Child so we can cancel it)
    let child = Command::new(rclone_path)
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| AppError::Rclone(format!("Failed to execute rclone: {}", e)))?;

    // Register the child in active_syncs so cancel_sync can find it
    let cancel_flag = {
        let mut map = active_syncs.lock().unwrap();
        let cancel = Arc::new(AtomicBool::new(false));
        map.insert(
            folder_id.to_string(),
            RunningSync {
                child: Arc::new(Mutex::new(child)),
                cancel_requested: cancel.clone(),
                started_at: Instant::now(),
            },
        );
        cancel
    };

    // Steal the child back out of the map so we can take ownership of its stdio pipes
    let child = {
        let map = active_syncs.lock().unwrap();
        let rs = map.get(folder_id).expect("just inserted");
        rs.child.lock().unwrap().try_wait().ok();
        // We can't move the child out of the Arc<Mutex<Child>>, so we'll read stdio
        // by holding the lock briefly to take the pipes, then wait on the child.
        // Instead: we keep a clone of the Arc<Mutex<Child>> and operate on the shared child.
        map.get(folder_id).unwrap().child.clone()
    };

    // Take stderr/stdout pipes out of the child
    let stderr = {
        let mut guard = child.lock().unwrap();
        guard.stderr.take()
    };
    let stdout = {
        let mut guard = child.lock().unwrap();
        guard.stdout.take()
    };

    // Spawn a stderr-reading thread that also emits progress events
    let folder_id_owned = folder_id.to_string();
    let app_handle_for_progress = app_handle.clone();
    let stderr_thread = thread::spawn(move || {
        let mut collected = String::new();
        let Some(stderr) = stderr else {
            return collected;
        };
        let reader = BufReader::new(stderr);
        for line in reader.lines().flatten() {
            collected.push_str(&line);
            collected.push('\n');
            maybe_emit_progress(&app_handle_for_progress, &folder_id_owned, &line);
        }
        collected
    });

    // Spawn a stdout-reading thread (just collect)
    let stdout_thread = thread::spawn(move || {
        let mut collected = String::new();
        let Some(stdout) = stdout else {
            return collected;
        };
        let reader = BufReader::new(stdout);
        for line in reader.lines().flatten() {
            collected.push_str(&line);
            collected.push('\n');
        }
        collected
    });

    // Wait loop: periodically check cancel flag
    let status = loop {
        if cancel_flag.load(Ordering::SeqCst) {
            // Kill the child; ignore errors (it may have already exited)
            let _ = child.lock().unwrap().kill();
            break None;
        }
        match child.lock().unwrap().try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) => {
                thread::sleep(std::time::Duration::from_millis(200));
                continue;
            }
            Err(e) => {
                log::error!("Error waiting on rclone child: {}", e);
                break None;
            }
        }
    };

    let stderr = stderr_thread.join().unwrap_or_default();
    let stdout = stdout_thread.join().unwrap_or_default();

    // Remove from active_syncs
    {
        let mut map = active_syncs.lock().unwrap();
        map.remove(folder_id);
    }

    if status.is_none() {
        // Process was cancelled or errored
        let duration_ms = start.elapsed().as_millis() as u64;
        log::info!(
            "rclone bisync cancelled/errored after {}ms for {}",
            duration_ms, local_path
        );
        if cancel_flag.load(Ordering::SeqCst) {
            // Emit a cancelled event to the frontend
            let _ = app_handle.emit("sync-progress", serde_json::json!({
                "folder_id": folder_id,
                "event": "cancelled",
            }));
            return Ok(cancelled_result(duration_ms));
        }
    }

    let duration_ms = start.elapsed().as_millis() as u64;
    let exit_code = status.and_then(|s| s.code()).unwrap_or(-1);

    log::info!("rclone bisync completed in {}ms (exit code: {})", duration_ms, exit_code);
    if !stderr.is_empty() {
        log::warn!("rclone stderr:\n{}", stderr);
    }
    if !stdout.is_empty() {
        log::debug!("rclone stdout:\n{}", stdout);
    }

    // Emit a "done" progress event so the frontend can clear any in-flight indicators
    let _ = app_handle.emit("sync-progress", serde_json::json!({
        "folder_id": folder_id,
        "event": "done",
    }));

    Ok(parse_bisync_output(&stdout, &stderr, exit_code, duration_ms))
}

/// Try to extract a progress line from rclone's verbose stderr and emit a
/// `sync-progress` Tauri event. We look for lines matching common rclone
/// progress formats (e.g. "Transferred: 1.2M / 5.3M, 22%").
fn maybe_emit_progress(app_handle: &AppHandle, folder_id: &str, line: &str) {
    // Match: "Transferred:            1.200 MiB / 5.300 MiB, 22%, 300.000 KiB/s, ETA 14s"
    if let Some(rest) = line.strip_prefix("Transferred:") {
        let rest = rest.trim();
        // Try to find percent
        if let Some(percent) = extract_percent(rest) {
            let _ = app_handle.emit(
                "sync-progress",
                serde_json::json!({
                    "folder_id": folder_id,
                    "event": "progress",
                    "percent": percent,
                    "raw": rest,
                }),
            );
        }
        return;
    }

    // Match file-level progress: rclone verbose lines like
    // `INFO  : path/to/file.jpg: Copied (new)` → emit as file event
    if line.contains(": Copied (") || line.contains(": Updated (") || line.contains(": Deleted") {
        if let Some(path) = extract_file_path(line) {
            let _ = app_handle.emit(
                "sync-progress",
                serde_json::json!({
                    "folder_id": folder_id,
                    "event": "file",
                    "file_path": path,
                }),
            );
        }
    }
}

fn extract_percent(s: &str) -> Option<u8> {
    // Find a standalone integer followed by "%"
    for part in s.split(|c: char| c.is_whitespace() || c == ',') {
        if let Some(num_str) = part.strip_suffix('%') {
            if let Ok(n) = num_str.trim().parse::<u8>() {
                if n <= 100 {
                    return Some(n);
                }
            }
        }
    }
    None
}

fn extract_file_path(line: &str) -> Option<String> {
    // Format: "2026/06/14 22:08:28 INFO  : path/to/file: Copied (new)"
    // Split on " : " to skip the timestamp + level prefix, then find the last colon
    let parts: Vec<&str> = line.splitn(2, " : ").collect();
    if parts.len() < 2 {
        return None;
    }
    let rest = parts[1];
    // Find the last " : " before the operation (Copied/Updated/Deleted)
    if let Some(idx) = rest.rfind(" : ") {
        let path = rest[..idx].trim();
        if !path.is_empty() {
            return Some(path.to_string());
        }
    }
    None
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
