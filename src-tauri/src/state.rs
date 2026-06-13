use rusqlite::Connection;
use std::collections::HashMap;
use std::sync::Mutex;

pub struct AppState {
    pub db: Mutex<Connection>,
    pub sync_locks: Mutex<HashMap<String, bool>>,
}
