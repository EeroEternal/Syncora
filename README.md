# Syncora

A cross-platform desktop file synchronization tool built with Tauri, using Cloudflare R2 as backend storage and rclone as the sync engine.

## Prerequisites

- [Node.js](https://nodejs.org/) >= 18
- [Rust](https://rustup.rs/) >= 1.77
- [rclone](https://rclone.org/) (downloaded automatically via script)

## Setup

```bash
# Install frontend dependencies
npm install

# Download rclone binary for your platform
node scripts/download-rclone.mjs

# Run in development mode
npm run tauri dev
```

## Tech Stack

- **Frontend:** SolidJS + Vite + Tailwind CSS v4 + Kobalte
- **Backend:** Tauri 2 + Rust + SQLite
- **Sync Engine:** rclone (bisync)
- **Storage:** Cloudflare R2

## Project Structure

```
├── src/                    # Frontend (SolidJS)
│   ├── components/         # UI components
│   ├── pages/              # Route pages
│   └── lib/                # Utilities & Tauri bindings
├── src-tauri/              # Rust backend
│   └── src/
│       ├── db/             # SQLite database layer
│       ├── rclone/         # rclone integration
│       ├── sync/           # Sync orchestrator
│       └── commands/       # Tauri IPC commands
└── scripts/                # Build scripts
```
