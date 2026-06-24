use std::collections::HashMap;
#[cfg(not(target_os = "android"))]
use std::path::Path;

use super::types::FileEntry;
use crate::error::AppError;

/// Recursively walk a local directory and collect all files.
/// Returns a map of relative_path → FileEntry.
#[cfg(not(target_os = "android"))]
pub fn walk_local_dir(root: &Path) -> Result<HashMap<String, FileEntry>, AppError> {
    let mut entries = HashMap::new();
    walk_recursive(root, root, &mut entries)?;
    Ok(entries)
}

#[cfg(not(target_os = "android"))]
fn walk_recursive(
    root: &Path,
    current: &Path,
    entries: &mut HashMap<String, FileEntry>,
) -> Result<(), AppError> {
    for entry in std::fs::read_dir(current)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            walk_recursive(root, &path, entries)?;
        } else if path.is_file() {
            if let Ok(rel) = path.strip_prefix(root) {
                let rel_str = rel.to_string_lossy().replace('\\', "/");
                let metadata = entry.metadata()?;
                let mtime = metadata
                    .modified()
                    .ok()
                    .map(|t| {
                        t.duration_since(std::time::UNIX_EPOCH)
                            .ok()
                            .map(|d| chrono::DateTime::from_timestamp(d.as_secs() as i64, 0))
                            .flatten()
                    })
                    .flatten();
                entries.insert(
                    rel_str,
                    FileEntry {
                        path: path.to_string_lossy().to_string(),
                        size: metadata.len(),
                        etag: None, // Local files don't have etags; use size+mtime
                        mtime,
                    },
                );
            }
        }
    }
    Ok(())
}

/// Read a local file's contents.
#[cfg(not(target_os = "android"))]
pub fn read_file(path: &Path) -> Result<Vec<u8>, AppError> {
    std::fs::read(path).map_err(AppError::from)
}

/// Write data to a local file, creating parent dirs as needed.
#[cfg(not(target_os = "android"))]
pub fn write_file(path: &Path, data: &[u8]) -> Result<(), AppError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, data).map_err(AppError::from)
}

/// Delete a local file.
#[cfg(not(target_os = "android"))]
pub fn delete_file(path: &Path) -> Result<(), AppError> {
    std::fs::remove_file(path).map_err(AppError::from)
}

// ── Android stubs (SAF-based access will be implemented later) ─────────────

#[cfg(target_os = "android")]
pub fn walk_local_dir(root: &str) -> Result<HashMap<String, FileEntry>, AppError> {
    Err(AppError::S3("SAF directory walk not yet implemented for Android".into()))
}

#[cfg(target_os = "android")]
pub fn read_file(path: &str) -> Result<Vec<u8>, AppError> {
    Err(AppError::S3("SAF file read not yet implemented for Android".into()))
}

#[cfg(target_os = "android")]
pub fn write_file(path: &str, data: &[u8]) -> Result<(), AppError> {
    Err(AppError::S3("SAF file write not yet implemented for Android".into()))
}

#[cfg(target_os = "android")]
pub fn delete_file(path: &str) -> Result<(), AppError> {
    Err(AppError::S3("SAF file delete not yet implemented for Android".into()))
}
