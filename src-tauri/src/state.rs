use rusqlite::Connection;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tokio::sync::Notify;

/// Represents a currently-running sync for a single folder.
/// On desktop, this wraps an rclone subprocess. On mobile (Android), there
/// is no subprocess — the S3 sync engine polls `cancel_requested` instead.
/// The presence of an entry in `active_syncs` acts as a "currently syncing" flag.
pub struct RunningSync {
    /// Set to `true` when a cancel request arrives. The sync task polls this
    /// flag periodically and stops if set.
    pub cancel_requested: Arc<AtomicBool>,
    pub started_at: Instant,
    /// Desktop only: the rclone subprocess handle, used for immediate kill.
    #[cfg(not(target_os = "android"))]
    pub child: Arc<Mutex<std::process::Child>>,
}

impl RunningSync {
    /// Request cancellation of this sync.
    /// On desktop, also kills the rclone subprocess immediately.
    /// On mobile, the S3 engine checks `cancel_requested` between transfers.
    pub fn cancel(&self) {
        self.cancel_requested.store(true, Ordering::SeqCst);
        #[cfg(not(target_os = "android"))]
        {
            if let Ok(mut c) = self.child.lock() {
                let _ = c.kill();
            }
        }
    }
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
