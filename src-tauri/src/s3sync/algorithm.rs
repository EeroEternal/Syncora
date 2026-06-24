use std::collections::HashMap;

use super::types::{FileEntry, SyncAction};

/// Compare local files, remote objects, and last-sync state to determine
/// the set of actions needed to bring both sides into sync.
///
/// # Arguments
/// * `local` - Files found on the local filesystem.
/// * `remote` - Objects found in the remote bucket (under the folder prefix).
/// * `last_state` - State recorded at the end of the previous successful sync.
///                  Empty for first-time sync (needs_resync).
///
/// # Returns
/// A map of file path → SyncAction.
pub fn compute_sync_actions(
    local: &HashMap<String, FileEntry>,
    remote: &HashMap<String, FileEntry>,
    last_state: &HashMap<String, FileEntry>,
) -> HashMap<String, SyncAction> {
    let mut actions = HashMap::new();

    // Collect all known paths from all three sides
    let all_paths: std::collections::HashSet<String> = local
        .keys()
        .chain(remote.keys())
        .chain(last_state.keys())
        .cloned()
        .collect();

    for path in all_paths {
        let local_entry = local.get(&path);
        let remote_entry = remote.get(&path);
        let state_entry = last_state.get(&path);

        let action = match (local_entry, remote_entry, state_entry) {
            // Not in last state — new file
            (Some(_), None, None) => SyncAction::UploadLocal,
            (None, Some(_), None) => SyncAction::DownloadRemote,
            (Some(l), Some(r), None) => {
                // Both new — conflict unless identical
                if entries_equal(l, r) {
                    SyncAction::Skip
                } else {
                    SyncAction::Conflict
                }
            }

            // In last state — check what changed
            (Some(l), Some(r), Some(s)) => {
                let local_changed = !entries_equal(l, s);
                let remote_changed = !entries_equal(r, s);
                match (local_changed, remote_changed) {
                    (false, false) => SyncAction::Skip,
                    (true, false) => SyncAction::UploadLocal,
                    (false, true) => SyncAction::DownloadRemote,
                    (true, true) => SyncAction::Conflict,
                }
            }

            // Deleted locally
            (None, Some(r), Some(s)) => {
                if entries_equal(r, s) {
                    SyncAction::DeleteRemote
                } else {
                    // Deleted locally but changed remotely — conflict
                    SyncAction::Conflict
                }
            }

            // Deleted remotely
            (Some(l), None, Some(s)) => {
                if entries_equal(l, s) {
                    SyncAction::DeleteLocal
                } else {
                    // Deleted remotely but changed locally — conflict
                    SyncAction::Conflict
                }
            }

            // Deleted on both sides — nothing to do
            (None, None, Some(_)) => SyncAction::Skip,
            (None, None, None) => SyncAction::Skip,
        };

        actions.insert(path, action);
    }

    actions
}

/// Compare two file entries for equality (by etag if available, otherwise size + mtime).
fn entries_equal(a: &FileEntry, b: &FileEntry) -> bool {
    if let (Some(ea), Some(eb)) = (&a.etag, &b.etag) {
        return ea == eb;
    }
    a.size == b.size
}
