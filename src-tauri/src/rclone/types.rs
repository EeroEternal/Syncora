use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncResult {
    pub success: bool,
    pub files_transferred: u32,
    pub files_deleted: u32,
    pub errors: Vec<String>,
    pub conflicts: Vec<ConflictInfo>,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictInfo {
    pub file_path: String,
    pub local_version: Option<String>,
    pub remote_version: Option<String>,
}

#[derive(Debug, Clone)]
pub enum SyncEvent {
    Started { folder_id: String },
    Progress { folder_id: String, message: String },
    Completed { folder_id: String, result: SyncResult },
    Error { folder_id: String, message: String },
}
