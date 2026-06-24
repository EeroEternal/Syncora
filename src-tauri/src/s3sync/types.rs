use serde::{Deserialize, Serialize};

/// A file entry representing either a local file or a remote S3 object.
#[derive(Debug, Clone)]
pub struct FileEntry {
    pub path: String,
    pub size: u64,
    pub etag: Option<String>,
    pub mtime: Option<chrono::DateTime<chrono::Utc>>,
}

/// Actions the sync algorithm can decide to take for a given file path.
#[derive(Debug, Clone, PartialEq)]
pub enum SyncAction {
    /// Upload local file to remote (new or modified locally).
    UploadLocal,
    /// Download remote object to local (new or modified remotely).
    DownloadRemote,
    /// Delete remote object (deleted locally since last sync).
    DeleteRemote,
    /// Delete local file (deleted remotely since last sync).
    DeleteLocal,
    /// Both sides changed since last sync — needs user resolution.
    Conflict,
    /// No action needed (both sides unchanged / in sync).
    Skip,
}

/// Result of a single sync operation for reporting.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncOutcome {
    pub success: bool,
    pub files_transferred: i32,
    pub files_deleted: i32,
    pub duration_ms: u64,
    pub errors: Vec<String>,
    pub transferred_paths: Vec<String>,
}
