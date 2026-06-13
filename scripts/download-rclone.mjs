#!/usr/bin/env node

/**
 * Script to download rclone binary for the current platform.
 * Renames to the Tauri sidecar naming convention: rclone-<target-triple>[.exe]
 *
 * Usage: node scripts/download-rclone.mjs
 */

import { execSync } from "child_process";
import { createWriteStream, existsSync, mkdirSync, renameSync, chmodSync, unlinkSync } from "fs";
import { join } from "path";
import https from "https";

const RCLONE_VERSION = "1.68.2";
const BINARIES_DIR = join(import.meta.dirname, "..", "src-tauri", "binaries");

function getPlatformInfo() {
  const platform = process.platform;
  const arch = process.arch;

  let os, rcloneArch, targetTriple, ext;

  switch (platform) {
    case "darwin":
      os = "osx";
      targetTriple = arch === "arm64" ? "aarch64-apple-darwin" : "x86_64-apple-darwin";
      rcloneArch = arch === "arm64" ? "arm64" : "amd64";
      ext = "";
      break;
    case "win32":
      os = "windows";
      targetTriple = "x86_64-pc-windows-msvc";
      rcloneArch = "amd64";
      ext = ".exe";
      break;
    case "linux":
      os = "linux";
      targetTriple = arch === "arm64" ? "aarch64-unknown-linux-gnu" : "x86_64-unknown-linux-gnu";
      rcloneArch = arch === "arm64" ? "arm64" : "amd64";
      ext = "";
      break;
    default:
      throw new Error(`Unsupported platform: ${platform}`);
  }

  return { os, rcloneArch, targetTriple, ext };
}

async function downloadFile(url, dest) {
  return new Promise((resolve, reject) => {
    const file = createWriteStream(dest);
    https.get(url, (response) => {
      if (response.statusCode === 302 || response.statusCode === 301) {
        // Follow redirect
        https.get(response.headers.location, (res) => {
          res.pipe(file);
          file.on("finish", () => { file.close(); resolve(); });
        }).on("error", reject);
      } else {
        response.pipe(file);
        file.on("finish", () => { file.close(); resolve(); });
      }
    }).on("error", reject);
  });
}

async function main() {
  const { os, rcloneArch, targetTriple, ext } = getPlatformInfo();

  console.log(`Platform: ${os}-${rcloneArch}`);
  console.log(`Target triple: ${targetTriple}`);

  if (!existsSync(BINARIES_DIR)) {
    mkdirSync(BINARIES_DIR, { recursive: true });
  }

  const outputName = `rclone-${targetTriple}${ext}`;
  const outputPath = join(BINARIES_DIR, outputName);

  if (existsSync(outputPath)) {
    console.log(`rclone binary already exists at: ${outputPath}`);
    console.log("Delete it manually if you want to re-download.");
    return;
  }

  const zipName = `rclone-v${RCLONE_VERSION}-${os}-${rcloneArch}.zip`;
  const url = `https://downloads.rclone.org/v${RCLONE_VERSION}/${zipName}`;
  const zipPath = join(BINARIES_DIR, zipName);

  console.log(`Downloading from: ${url}`);
  await downloadFile(url, zipPath);
  console.log("Download complete. Extracting...");

  // Extract
  execSync(`unzip -o "${zipPath}" -d "${BINARIES_DIR}"`, { stdio: "pipe" });

  // Find and move binary
  const extractedDir = join(BINARIES_DIR, `rclone-v${RCLONE_VERSION}-${os}-${rcloneArch}`);
  const extractedBinary = join(extractedDir, `rclone${ext}`);
  renameSync(extractedBinary, outputPath);
  chmodSync(outputPath, 0o755);

  // Cleanup
  execSync(`rm -rf "${extractedDir}" "${zipPath}"`, { stdio: "pipe" });

  console.log(`✅ rclone binary saved to: ${outputPath}`);
}

main().catch((err) => {
  console.error("Error:", err.message);
  process.exit(1);
});
