/// Foreground service plugin for Android.
///
/// On Android, we need a foreground service with a persistent notification
/// to keep the sync running without being killed by the OS. This module
/// provides a Tauri plugin that bridges Rust ↔ Kotlin for starting/stopping
/// the service.
///
/// On desktop, all functions are no-ops.

#[cfg(target_os = "android")]
use std::sync::OnceLock;

#[cfg(target_os = "android")]
use tauri::plugin::{Builder, PluginHandle, TauriPlugin};

#[cfg(target_os = "android")]
static FOREGROUND_HANDLE: OnceLock<PluginHandle<tauri::Wry>> = OnceLock::new();

/// Initialize the foreground service plugin (Android only).
#[cfg(target_os = "android")]
pub fn init() -> TauriPlugin<tauri::Wry> {
    Builder::new("foreground-service")
        .setup(|_app, api| {
            let handle = api.register_android_plugin(
                "com.syncora.app",
                "ForegroundServicePlugin",
            )?;
            let _ = FOREGROUND_HANDLE.set(handle);
            Ok(())
        })
        .build()
}

/// Start the foreground service with a notification showing the folder being synced.
#[cfg(target_os = "android")]
pub fn start_foreground(folder_name: &str) {
    use serde::Serialize;

    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct StartForegroundPayload {
        folder_name: String,
    }

    if let Some(handle) = FOREGROUND_HANDLE.get() {
        let payload = StartForegroundPayload {
            folder_name: folder_name.to_string(),
        };
        if let Err(e) = handle.run_mobile_plugin::<serde_json::Value>(
            "startForeground",
            payload,
        ) {
            log::warn!("Failed to start foreground service: {}", e);
        }
    }
}

/// Stop the foreground service.
#[cfg(target_os = "android")]
pub fn stop_foreground() {
    if let Some(handle) = FOREGROUND_HANDLE.get() {
        if let Err(e) = handle.run_mobile_plugin::<serde_json::Value>(
            "stopForeground",
            serde_json::json!({}),
        ) {
            log::warn!("Failed to stop foreground service: {}", e);
        }
    }
}

/// Desktop no-op.
#[cfg(not(target_os = "android"))]
pub fn start_foreground(_folder_name: &str) {}

/// Desktop no-op.
#[cfg(not(target_os = "android"))]
pub fn stop_foreground() {}
