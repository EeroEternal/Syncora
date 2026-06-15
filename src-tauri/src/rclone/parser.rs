use super::types::{ConflictInfo, SyncResult};

/// Parse rclone bisync output to extract sync results.
/// rclone --verbose emits lines like:
///   "INFO  : path/to/file.txt: Copied (new)"
///   "INFO  : path/to/file.txt: Updated"
///   "INFO  : path/to/file.txt: Deleted"
///   "Transferred: N items, ..."
pub fn parse_bisync_output(stdout: &str, stderr: &str, exit_code: i32, duration_ms: u64) -> SyncResult {
    let mut result = SyncResult {
        success: exit_code == 0,
        files_transferred: 0,
        files_deleted: 0,
        transferred_paths: Vec::new(),
        errors: Vec::new(),
        conflicts: Vec::new(),
        duration_ms,
    };

    let combined = format!("{}\n{}", stdout, stderr);

    for line in combined.lines() {
        // ── Conflict detection ──────────────────────────────────────────────
        if line.contains("WARNING") && line.contains("New or changed in both paths") {
            if let Some(file_path) = extract_conflict_path(line) {
                result.conflicts.push(ConflictInfo {
                    file_path,
                    local_version: None,
                    remote_version: None,
                });
            }
        }

        // ── Individual file transfer lines ──────────────────────────────────
        // rclone verbose: "INFO  : relative/path/file.ext: Copied (new)"
        //                 "INFO  : relative/path/file.ext: Updated"
        //                 "INFO  : relative/path/file.ext: Deleted"
        if (line.contains(": Copied") || line.contains(": Updated") || line.contains(": Moved"))
            && line.contains("INFO")
        {
            if let Some(path) = extract_file_path(line) {
                result.files_transferred += 1;
                result.transferred_paths.push(path);
            }
        } else if line.contains(": Deleted") && line.contains("INFO") {
            if let Some(path) = extract_file_path(line) {
                result.files_deleted += 1;
                result.transferred_paths.push(format!("(deleted) {}", path));
            }
        }

        // ── Summary line fallback (in case verbose flag changes) ─────────────
        // "Transferred:   3 / 3, 100%"
        if line.trim_start().starts_with("Transferred:") && !line.contains("Bytes") {
            if let Some(n) = parse_transferred_count(line) {
                // Only use if we didn't parse individual lines
                if result.files_transferred == 0 {
                    result.files_transferred = n;
                }
            }
        }

        // ── Errors ──────────────────────────────────────────────────────────
        if (line.contains("ERROR") || line.contains("Failed to")) && !line.contains("NOTICE") {
            let trimmed = line.trim().to_string();
            if !trimmed.is_empty() {
                result.errors.push(trimmed);
            }
        }

        // ── Explicit success marker ──────────────────────────────────────────
        if line.contains("Bisync successful") {
            result.success = true;
        }
    }

    if !result.errors.is_empty() && result.conflicts.is_empty() {
        result.success = false;
    }

    result
}

/// Extract file path from a rclone INFO transfer line.
/// Input:  "2026/06/14 12:30:00 INFO  : relative/path/file.txt: Copied (new)"
/// Output: "relative/path/file.txt"
fn extract_file_path(line: &str) -> Option<String> {
    // Find " : " separator after log prefix, then take up to the last ":"
    let after_info = line.split(" : ").nth(1)?;
    // Everything before the last ": <action>" is the path
    let colon_pos = after_info.rfind(':')?;
    let path = after_info[..colon_pos].trim().to_string();
    if path.is_empty() { None } else { Some(path) }
}

/// Parse the transferred item count from a summary line like
/// "Transferred:   3 / 3, 100%"
fn parse_transferred_count(line: &str) -> Option<u32> {
    // "Transferred:   3 / 3, 100%" → 3
    let after = line.splitn(2, ':').nth(1)?.trim();
    let first_token = after.split_whitespace().next()?;
    first_token.parse::<u32>().ok()
}

fn extract_conflict_path(line: &str) -> Option<String> {
    // Pattern: "NOTICE: - WARNING  New or changed in both paths        - filename.txt"
    if let Some(idx) = line.rfind(" - ") {
        let path = line[idx + 3..].trim().to_string();
        if !path.is_empty() {
            return Some(path);
        }
    }
    None
}
