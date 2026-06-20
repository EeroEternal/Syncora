import { invoke } from "@tauri-apps/api/core";

// Auth error event — any invoke that gets a session-expired error
// fires this so the App layout can redirect to /login.
export const AUTH_EXPIRED_EVENT = "syncora:auth-expired";

function isAuthError(err: unknown): boolean {
  const msg = String((err as any)?.message ?? err ?? "");
  return (
    msg.includes("Session expired") ||
    msg.includes("Authentication error") ||
    msg.includes("Not logged in") ||
    msg.includes("Unauthorized")
  );
}

async function invokeAuth<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(cmd, args);
  } catch (err) {
    if (isAuthError(err)) {
      window.dispatchEvent(new CustomEvent(AUTH_EXPIRED_EVENT));
    }
    throw err;
  }
}

// Types
export interface Folder {
  id: string;
  local_path: string;
  remote_prefix: string;
  mode: "normal" | "cloud_only";
  last_sync_at: string | null;
  status: "idle" | "syncing" | "synced" | "error" | "released";
  is_enabled: boolean;
  created_at: string;
}

export interface Conflict {
  id: string;
  folder_id: string;
  file_path: string;
  local_version: string | null;
  remote_version: string | null;
  detected_at: string;
  resolved: boolean;
  resolution: string | null;
}

export interface SyncLog {
  id: string;
  folder_id: string;
  timestamp: string;
  action: string;
  status: "success" | "error" | "warning";
  message: string | null;
  duration_ms: number | null;
}

export interface Settings {
  api_base_url: string;
  sync_interval_minutes: number;
  auto_start: boolean;
}

export interface UserInfo {
  id: string;
  email: string;
  display_name: string | null;
}

export interface AuthStatus {
  logged_in: boolean;
  user: UserInfo | null;
}

// Auth commands
export async function register(email: string, password: string): Promise<UserInfo> {
  return invoke("register", { email, password });
}

export async function login(email: string, password: string): Promise<UserInfo> {
  return invoke("login", { email, password });
}

export async function logout(): Promise<void> {
  return invoke("logout");
}

export async function getAuthStatus(): Promise<AuthStatus> {
  return invoke("get_auth_status");
}

// Settings commands
export async function getSettings(): Promise<Settings> {
  return invoke("get_settings");
}

export async function saveSettings(settings: Settings): Promise<void> {
  return invoke("save_settings", { settings });
}

// Folder commands
export async function listFolders(): Promise<Folder[]> {
  return invokeAuth("list_folders");
}

export async function addFolder(localPath: string, mode?: "create" | "connect" | "replace"): Promise<Folder> {
  return invokeAuth("add_folder", { localPath, mode });
}

export async function deleteFolder(id: string): Promise<void> {
  return invokeAuth("delete_folder", { id });
}

// Open a folder in the OS file explorer
export async function openFolder(path: string): Promise<void> {
  return invoke("open_folder", { path });
}

// Sync commands
export async function triggerSync(folderId: string): Promise<void> {
  return invokeAuth("trigger_sync", { folderId });
}

export async function triggerSyncAll(): Promise<void> {
  return invokeAuth("trigger_sync_all");
}

export async function cancelSync(folderId: string): Promise<void> {
  return invoke("cancel_sync", { folderId });
}

// Conflict commands
export async function listConflicts(resolved?: boolean): Promise<Conflict[]> {
  return invokeAuth("list_conflicts", { resolved });
}

export async function resolveConflict(
  id: string,
  resolution: "keep_local" | "keep_remote" | "keep_both"
): Promise<void> {
  return invokeAuth("resolve_conflict", { id, resolution });
}

// Log commands
export async function getLogs(folderId?: string, limit?: number): Promise<SyncLog[]> {
  return invokeAuth("get_logs", { folderId, limit });
}

export async function getRecentActivity(limit?: number): Promise<SyncLog[]> {
  return invokeAuth("get_recent_activity", { limit });
}

// Release commands
export async function releaseFolder(folderId: string): Promise<void> {
  return invokeAuth("release_folder", { folderId });
}
