// Desktop-only: rclone subprocess management
#[cfg(not(target_os = "android"))]
pub mod config;
#[cfg(not(target_os = "android"))]
pub mod bisync;
#[cfg(not(target_os = "android"))]
pub mod parser;

// Shared: sync result types used by both desktop (rclone) and mobile (S3 engine)
pub mod types;

pub use types::*;
