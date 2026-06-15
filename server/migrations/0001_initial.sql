-- Syncora D1 Schema - Initial Migration

-- Users table
CREATE TABLE users (
  id TEXT PRIMARY KEY,
  email TEXT NOT NULL UNIQUE,
  password_hash TEXT NOT NULL,
  display_name TEXT,
  r2_prefix TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT (datetime('now')),
  updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE UNIQUE INDEX idx_users_email ON users(email);

-- Folders table (per-user)
CREATE TABLE folders (
  id TEXT PRIMARY KEY,
  user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  local_path_hint TEXT,
  remote_prefix TEXT NOT NULL,
  mode TEXT NOT NULL DEFAULT 'normal' CHECK(mode IN ('normal','cloud_only')),
  last_sync_at TEXT,
  status TEXT NOT NULL DEFAULT 'idle' CHECK(status IN ('idle','syncing','synced','error','released')),
  is_enabled INTEGER NOT NULL DEFAULT 1,
  needs_resync INTEGER NOT NULL DEFAULT 1,
  created_at TEXT NOT NULL DEFAULT (datetime('now')),
  updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_folders_user_id ON folders(user_id);

-- Conflicts table
CREATE TABLE conflicts (
  id TEXT PRIMARY KEY,
  user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  folder_id TEXT NOT NULL REFERENCES folders(id) ON DELETE CASCADE,
  file_path TEXT NOT NULL,
  local_version TEXT,
  remote_version TEXT,
  detected_at TEXT NOT NULL DEFAULT (datetime('now')),
  resolved INTEGER NOT NULL DEFAULT 0,
  resolution TEXT CHECK(resolution IN ('keep_local','keep_remote','keep_both'))
);

CREATE INDEX idx_conflicts_user_id ON conflicts(user_id);
CREATE INDEX idx_conflicts_folder_id ON conflicts(folder_id);

-- Sync logs table
CREATE TABLE sync_logs (
  id TEXT PRIMARY KEY,
  user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  folder_id TEXT NOT NULL REFERENCES folders(id) ON DELETE CASCADE,
  timestamp TEXT NOT NULL DEFAULT (datetime('now')),
  action TEXT NOT NULL,
  status TEXT NOT NULL CHECK(status IN ('success','error','warning')),
  message TEXT,
  duration_ms INTEGER,
  files_transferred INTEGER DEFAULT 0,
  files_deleted INTEGER DEFAULT 0
);

CREATE INDEX idx_sync_logs_user_id ON sync_logs(user_id);
CREATE INDEX idx_sync_logs_folder_id ON sync_logs(folder_id);

-- User settings (server-side preferences)
CREATE TABLE user_settings (
  user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  key TEXT NOT NULL,
  value TEXT NOT NULL,
  PRIMARY KEY (user_id, key)
);

-- Refresh tokens (for JWT rotation)
CREATE TABLE refresh_tokens (
  id TEXT PRIMARY KEY,
  user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  token_hash TEXT NOT NULL,
  expires_at TEXT NOT NULL,
  revoked INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_refresh_tokens_user_id ON refresh_tokens(user_id);
CREATE INDEX idx_refresh_tokens_token_hash ON refresh_tokens(token_hash);
