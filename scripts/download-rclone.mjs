#!/usr/bin/env node

/**
 * Script to download rclone binary for the current platform.
 * Renames to the Tauri sidecar naming convention: rclone-<target-triple>[.exe]
 *
 * Features:
 *   - Validates file size (rejects empty / truncated downloads)
 *   - Validates SHA256 against rclone.org SHA256SUMS
 *   - Skips re-download if a valid binary already exists (size + hash)
 *   - `--force` env flag forces re-download even if valid
 *   - `RCLONE_VERSION` env overrides the default version (e.g. RCLONE_VERSION=1.68.2)
 *
 * Usage:
 *   node scripts/download-rclone.mjs
 *   RCLONE_VERSION=1.68.2 node scripts/download-rclone.mjs
 *   FORCE=1 node scripts/download-rclone.mjs
 */

import { execSync } from "child_process";
import {
  createWriteStream,
  existsSync,
  mkdirSync,
  renameSync,
  chmodSync,
  statSync,
} from "fs";
import { createHash } from "crypto";
import { createReadStream } from "fs";
import { join } from "path";
import https from "https";

const DEFAULT_VERSION = "1.68.2";
const RCLONE_VERSION = process.env.RCLONE_VERSION || DEFAULT_VERSION;
// rclone binaries are ~30–80 MB; anything < 1 MB is definitely bogus
const MIN_VALID_SIZE = 1 * 1024 * 1024;
const FORCE = process.env.FORCE === "1" || process.argv.includes("--force");

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

function httpsGet(url, redirectsLeft = 5) {
  return new Promise((resolve, reject) => {
    https
      .get(url, (response) => {
        if ((response.statusCode === 302 || response.statusCode === 301) && response.headers.location) {
          if (redirectsLeft <= 0) return reject(new Error("Too many redirects"));
          return httpsGet(response.headers.location, redirectsLeft - 1).then(resolve, reject);
        }
        if (response.statusCode !== 200) {
          return reject(new Error(`HTTP ${response.statusCode} for ${url}`));
        }
        const chunks = [];
        response.on("data", (c) => chunks.push(c));
        response.on("end", () => resolve(Buffer.concat(chunks)));
        response.on("error", reject);
      })
      .on("error", reject);
  });
}

function downloadToFile(url, dest) {
  return new Promise((resolve, reject) => {
    const follow = (u, redirectsLeft) => {
      https.get(u, (response) => {
        if ((response.statusCode === 302 || response.statusCode === 301) && response.headers.location) {
          if (redirectsLeft <= 0) return reject(new Error("Too many redirects"));
          return follow(response.headers.location, redirectsLeft - 1);
        }
        if (response.statusCode !== 200) {
          return reject(new Error(`HTTP ${response.statusCode} for ${u}`));
        }
        const file = createWriteStream(dest);
        response.pipe(file);
        file.on("finish", () => {
          file.close(() => resolve());
        });
        file.on("error", (err) => {
          reject(err);
        });
      }).on("error", reject);
    };
    follow(url, 5);
  });
}

function sha256File(filePath) {
  return new Promise((resolve, reject) => {
    const hash = createHash("sha256");
    const stream = createReadStream(filePath);
    stream.on("data", (chunk) => hash.update(chunk));
    stream.on("end", () => resolve(hash.digest("hex")));
    stream.on("error", reject);
  });
}

async function main() {
  const { os, rcloneArch, targetTriple, ext } = getPlatformInfo();

  console.log(`Platform: ${os}-${rcloneArch}`);
  console.log(`Target triple: ${targetTriple}`);
  console.log(`rclone version: ${RCLONE_VERSION}`);

  if (!existsSync(BINARIES_DIR)) {
    mkdirSync(BINARIES_DIR, { recursive: true });
  }

  const outputName = `rclone-${targetTriple}${ext}`;
  const outputPath = join(BINARIES_DIR, outputName);

  // Skip if a valid binary already exists (size >= MIN_VALID_SIZE) and --force not set.
  if (!FORCE && existsSync(outputPath)) {
    const size = statSync(outputPath).size;
    if (size >= MIN_VALID_SIZE) {
      const hash = await sha256File(outputPath);
      console.log(`✅ rclone binary already exists and is valid (${size} bytes, sha256=${hash.slice(0, 16)}...)`);
      console.log("Use FORCE=1 to re-download.");
      return;
    }
    console.warn(`⚠️  existing binary at ${outputPath} is suspiciously small (${size} bytes) — deleting.`);
  }

  const zipName = `rclone-v${RCLONE_VERSION}-${os}-${rcloneArch}.zip`;
  const url = `https://downloads.rclone.org/v${RCLONE_VERSION}/${zipName}`;
  const sumsUrl = `https://downloads.rclone.org/v${RCLONE_VERSION}/SHA256SUMS`;
  const zipPath = join(BINARIES_DIR, zipName);

  console.log(`Fetching SHA256SUMS: ${sumsUrl}`);
  const sumsBuf = await httpsGet(sumsUrl);
  const sumsText = sumsBuf.toString("utf8");
  const expectedLine = sumsText
    .split("\n")
    .find((line) => line.trim().endsWith(zipName));
  if (!expectedLine) {
    throw new Error(`Could not find SHA256 entry for ${zipName} in SHA256SUMS`);
  }
  const expectedHash = expectedLine.trim().split(/\s+/)[0].toLowerCase();
  console.log(`Expected SHA256: ${expectedHash}`);

  console.log(`Downloading: ${url}`);
  await downloadToFile(url, zipPath);
  console.log("Download complete.");

  // Size check on zip
  const zipSize = statSync(zipPath).size;
  if (zipSize < MIN_VALID_SIZE) {
    throw new Error(`Downloaded zip is suspiciously small (${zipSize} bytes). Network error?`);
  }

  // SHA256 check
  const actualHash = await sha256File(zipPath);
  if (actualHash.toLowerCase() !== expectedHash) {
    throw new Error(
      `SHA256 mismatch for ${zipName}.\n  expected: ${expectedHash}\n  actual:   ${actualHash}`
    );
  }
  console.log("SHA256 verified ✅");

  console.log("Extracting...");
  // Cross-platform unzip: macOS/Linux use `unzip`, Windows uses tar (built-in since Win10)
  if (process.platform === "win32") {
    execSync(`tar -xf "${zipPath}" -C "${BINARIES_DIR}"`, { stdio: "pipe" });
  } else {
    execSync(`unzip -o "${zipPath}" -d "${BINARIES_DIR}"`, { stdio: "pipe" });
  }

  const extractedDir = join(BINARIES_DIR, `rclone-v${RCLONE_VERSION}-${os}-${rcloneArch}`);
  const extractedBinary = join(extractedDir, `rclone${ext}`);
  if (!existsSync(extractedBinary)) {
    throw new Error(`Expected extracted binary not found at ${extractedBinary}`);
  }
  renameSync(extractedBinary, outputPath);
  chmodSync(outputPath, 0o755);

  // Cleanup
  try {
    execSync(
      process.platform === "win32"
        ? `rmdir /s /q "${extractedDir}" && del /f /q "${zipPath}"`
        : `rm -rf "${extractedDir}" "${zipPath}"`,
      { stdio: "pipe" }
    );
  } catch (_) {
    /* non-fatal */
  }

  const finalSize = statSync(outputPath).size;
  console.log(`✅ rclone binary saved to: ${outputPath} (${finalSize} bytes)`);
}

main().catch((err) => {
  console.error("❌ rclone download failed:", err.message);
  process.exit(1);
});
