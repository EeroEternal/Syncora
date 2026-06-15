use rusqlite::Connection;
use std::collections::HashMap;
use std::process::Child;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tokio::sync::Notify;

/// Represents a currently-running rclone bisync subprocess for a single folder.
/// The presence of an entry in `active_syncs` also acts as a "currently syncing" flag,
/// so no separate sync_locks structure is needed.
pub struct RunningSync {
    pub child: Arc<Mutex<Child>>,
    /// Set to `true` when a cancel request arrives. The task running the sync
    /// polls this flag periodically and, if true, kills the child.
    pub cancel_requested: Arc<AtomicBool>,
    pub started_at: Instant,
}

pub struct AppState {
    pub db: Arc<Mutex<Connection>>,
    /// Currently running rclone bisync processes, keyed by folder_id.
    /// Used both for deduplication (prevent parallel sync of the same folder)
    /// and for user-initiated cancellation.
    pub active_syncs: Arc<Mutex<HashMap<String, RunningSync>>>,
    pub api_base_url: String,
    /// Signal to trigger an immediate sync cycle
    pub sync_notify: Arc<Notify>,
}
