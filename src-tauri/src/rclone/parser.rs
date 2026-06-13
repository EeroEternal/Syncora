use super::types::{ConflictInfo, SyncResult};

/// Parse rclone bisync output to extract sync results
pub fn parse_bisync_output(stdout: &str, stderr: &str, exit_code: i32, duration_ms: u64) -> SyncResult {
    let mut result = SyncResult {
        success: exit_code == 0,
        files_transferred: 0,
        files_deleted: 0,
        errors: Vec::new(),
        conflicts: Vec::new(),
        duration_ms,
    };

    let combined = format!("{}\n{}", stdout, stderr);

    for line in combined.lines() {
        // Detect conflicts
        if line.contains("WARNING") && line.contains("New or changed in both paths") {
            if let Some(file_path) = extract_conflict_path(line) {
                result.conflicts.push(ConflictInfo {
                    file_path,
                    local_version: None,
                    remote_version: None,
                });
            }
        }

        // Count transfers
        if line.contains("Transferred:") && line.contains("Bytes") {
            // Parse transfer count from summary line
        }

        // Detect errors
        if line.contains("ERROR") || line.contains("Failed to") {
            result.errors.push(line.trim().to_string());
        }

        // Check for success
        if line.contains("Bisync successful") {
            result.success = true;
        }
    }

    if !result.errors.is_empty() && result.conflicts.is_empty() {
        result.success = false;
    }

    result
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
