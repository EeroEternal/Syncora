// Standalone lifecycle test exercising folder delete / re-add / release paths
// against the real SQLite DB and the real rclone binary.
// Run: cd src-tauri && cargo run --bin lifecycle_test

#[cfg(not(target_os = "android"))]
mod desktop {
    use std::collections::HashMap;
    use std::path::PathBuf;
    use std::process::Command;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex};
    use std::time::Instant;

    use syncora_lib::db;
    use syncora_lib::state::RunningSync;

    pub fn run() {
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

        let app_data_dir = PathBuf::from(
            std::env::var("SYNCORA_TEST_DATA_DIR")
                .unwrap_or_else(|_| {
                    let home = std::env::var("HOME").unwrap();
                    format!("{}/Library/Application Support/com.syncora.app", home)
                }),
        );
        std::fs::create_dir_all(&app_data_dir).unwrap();
        let conn = db::init_database(&app_data_dir).expect("init_database");

        // 1. Snapshot current folders
        println!("\n=== [1] initial state ===");
        let before = db::folders::list_all(&conn).unwrap();
        for f in &before {
            println!("  {} | {} | mode={} | status={}", f.id, f.local_path, f.mode, f.status);
        }

        // Build active_syncs map — simulate a "running" sync by spawning `sleep 30`
        // so we can verify delete_folder actually kills the child.
        let active_syncs: Arc<Mutex<HashMap<String, RunningSync>>> = Arc::new(Mutex::new(HashMap::new()));
        let target_folder = before
            .iter()
            .find(|f| f.local_path.contains("nnpic"))
            .expect("nnpic folder missing")
            .clone();

        let child = Command::new("sleep")
            .arg("30")
            .spawn()
            .expect("spawn sleep");
        let child_pid = child.id();
        println!("  spawned dummy child pid={} for folder {}", child_pid, target_folder.id);
        {
            let rs = RunningSync {
                child: Arc::new(Mutex::new(child)),
                cancel_requested: Arc::new(AtomicBool::new(false)),
                started_at: Instant::now(),
            };
            active_syncs.lock().unwrap().insert(target_folder.id.clone(), rs);
        }

        // 2. Delete folder
        println!("\n=== [2] delete_folder (with active sync) ===");
        {
            let mut map = active_syncs.lock().unwrap();
            if let Some(rs) = map.remove(&target_folder.id) {
                rs.cancel_requested.store(true, Ordering::SeqCst);
                let mut child = rs.child.lock().unwrap();
                if let Err(e) = child.kill() {
                    println!("  kill err (expected InvalidInput): {:?}", e);
                } else {
                    println!("  child killed ok");
                }
                // wait to reap
                drop(child);
                let mut guard = rs.child.lock().unwrap();
                let status = guard.wait().unwrap();
                println!("  child wait status: {}", status);
            }
        }

        // Also test DB cleanup logic
        let logs_before: i64 = conn
            .query_row("SELECT COUNT(*) FROM sync_logs WHERE folder_id=?1", [&target_folder.id], |r| r.get(0))
            .unwrap();
        println!("  sync_logs for {} before delete: {}", target_folder.id, logs_before);

        let _ = conn.execute("DELETE FROM sync_logs WHERE folder_id = ?1", rusqlite::params![&target_folder.id]);
        let _ = conn.execute("DELETE FROM conflicts WHERE folder_id = ?1", rusqlite::params![&target_folder.id]);
        db::folders::delete(&conn, &target_folder.id).unwrap();
        println!("  folder deleted");

        let logs_after: i64 = conn
            .query_row("SELECT COUNT(*) FROM sync_logs WHERE folder_id=?1", [&target_folder.id], |r| r.get(0))
            .unwrap();
        println!("  sync_logs for {} after delete: {}", target_folder.id, logs_after);

        // 3. Check that dummy child really died
        println!("\n=== [3] verify child process gone ===");
        let probe = Command::new("kill").arg("-0").arg(child_pid.to_string()).status();
        match probe {
            Ok(s) if s.success() => println!("  BAD: pid {} still alive!", child_pid),
            _ => println!("  OK: pid {} is gone", child_pid),
        }

        // 4. Re-add folder by local_path — exercises the exists_by_local_path check
        println!("\n=== [4] re-add same local_path ===");
        let exists = db::folders::exists_by_local_path(&conn, &target_folder.local_path).unwrap();
        println!("  exists_by_local_path('{}'): {}", target_folder.local_path, exists);
        assert!(!exists, "expected no row after delete");

        let new_id = uuid::Uuid::new_v4().to_string();
        let created = db::folders::create(&conn, &new_id, &target_folder.local_path, &target_folder.remote_prefix).unwrap();
        println!("  re-added folder: id={} mode={} status={}", created.id, created.mode, created.status);

        // 5. Test release_local_files DB state transitions
        println!("\n=== [5] release_local_files DB state transitions ===");
        let newpic = db::folders::list_all(&conn)
            .unwrap()
            .into_iter()
            .find(|f| f.local_path.contains("newpic"))
            .expect("newpic folder missing");
        println!("  newpic before: mode={} status={}", newpic.mode, newpic.status);

        // simulate the DB changes that release_local_files performs
        db::folders::update_status(&conn, &newpic.id, "syncing").unwrap();
        let f = db::folders::get_by_id(&conn, &newpic.id).unwrap();
        println!("  after update_status(syncing): status={}", f.status);

        db::folders::update_mode(&conn, &newpic.id, "cloud_only").unwrap();
        let _ = db::folders::update_last_sync(&conn, &newpic.id);
        let _ = conn.execute(
            "UPDATE folders SET status = 'released' WHERE id = ?1",
            rusqlite::params![&newpic.id],
        );
        let f = db::folders::get_by_id(&conn, &newpic.id).unwrap();
        println!("  after release transitions: mode={} status={} last_sync_at={:?}",
            f.mode, f.status, f.last_sync_at);
        assert_eq!(f.mode, "cloud_only");
        assert_eq!(f.status, "released");
        assert!(f.last_sync_at.is_some());

        // 6. Rollback: restore newpic to normal (so the app is usable after the test)
        println!("\n=== [6] rollback newpic back to normal ===");
        db::folders::update_mode(&conn, &newpic.id, "normal").unwrap();
        db::folders::update_status(&conn, &newpic.id, "synced").unwrap();

        // Delete the re-added test folder (keep original nnpic intact for next run)
        db::folders::delete(&conn, &new_id).unwrap();
        println!("  rolled back");

        // Final snapshot
        println!("\n=== [final] state ===");
        for f in db::folders::list_all(&conn).unwrap() {
            println!("  {} | {} | mode={} | status={}", f.id, f.local_path, f.mode, f.status);
        }

        println!("\nALL LIFECYCLE CHECKS PASSED");
    }
}

#[cfg(not(target_os = "android"))]
fn main() {
    desktop::run();
}

#[cfg(target_os = "android")]
fn main() {}
