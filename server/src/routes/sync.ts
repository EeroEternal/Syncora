import { Hono } from "hono";
import { z } from "zod";
import { zValidator } from "@hono/zod-validator";
import type { Env } from "../env";
import { authMiddleware, type AuthContext } from "../middleware/auth";
import { generateId } from "../utils/id";

const sync = new Hono<{ Bindings: Env; Variables: AuthContext }>();

// All sync routes require authentication
sync.use("*", authMiddleware);

// POST /sync/credentials - Get R2 credentials for rclone
const credentialsSchema = z.object({
  folder_id: z.string(),
});

sync.post("/credentials", zValidator("json", credentialsSchema), async (c) => {
  const userId = c.get("user").sub;
  const { folder_id } = c.req.valid("json");
  const db = c.env.DB;

  // Verify folder ownership
  const folder = await db
    .prepare("SELECT id, remote_prefix, needs_resync FROM folders WHERE id = ?1 AND user_id = ?2")
    .bind(folder_id, userId)
    .first<{ id: string; remote_prefix: string; needs_resync: number }>();

  if (!folder) {
    return c.json({ error: "Folder not found" }, 404);
  }

  // Update status to syncing
  await db
    .prepare("UPDATE folders SET status = 'syncing', updated_at = datetime('now') WHERE id = ?1")
    .bind(folder_id)
    .run();

  // Construct the full remote path: user_id/remote_prefix
  const remotePath = `${userId}/${folder.remote_prefix}`;

  return c.json({
    access_key_id: c.env.R2_ACCESS_KEY_ID,
    secret_access_key: c.env.R2_SECRET_ACCESS_KEY,
    endpoint: c.env.R2_ENDPOINT,
    bucket: c.env.R2_BUCKET_NAME,
    remote_path: remotePath,
    needs_resync: folder.needs_resync === 1,
  });
});

// POST /sync/report - Report sync result
const reportSchema = z.object({
  folder_id: z.string(),
  success: z.boolean(),
  files_transferred: z.number().optional().default(0),
  files_deleted: z.number().optional().default(0),
  duration_ms: z.number().optional().default(0),
  errors: z.array(z.string()).optional().default([]),
  conflicts: z
    .array(
      z.object({
        file_path: z.string(),
        local_version: z.string().nullable().optional(),
        remote_version: z.string().nullable().optional(),
      })
    )
    .optional()
    .default([]),
});

sync.post("/report", zValidator("json", reportSchema), async (c) => {
  const userId = c.get("user").sub;
  const report = c.req.valid("json");
  const db = c.env.DB;

  // Verify folder ownership
  const folder = await db
    .prepare("SELECT id FROM folders WHERE id = ?1 AND user_id = ?2")
    .bind(report.folder_id, userId)
    .first();

  if (!folder) {
    return c.json({ error: "Folder not found" }, 404);
  }

  // Update folder status
  const logStatus = report.success ? "success" : "error";
  if (report.success) {
    await db
      .prepare(
        "UPDATE folders SET status = 'synced', last_sync_at = datetime('now'), needs_resync = 0, updated_at = datetime('now') WHERE id = ?1"
      )
      .bind(report.folder_id)
      .run();
  } else {
    await db
      .prepare("UPDATE folders SET status = 'error', updated_at = datetime('now') WHERE id = ?1")
      .bind(report.folder_id)
      .run();
  }

  // Insert sync log
  const logId = generateId();
  const errorMessage = report.errors.length > 0 ? report.errors[0] : null;
  await db
    .prepare(
      "INSERT INTO sync_logs (id, user_id, folder_id, action, status, message, duration_ms, files_transferred, files_deleted) VALUES (?1, ?2, ?3, 'bisync', ?4, ?5, ?6, ?7, ?8)"
    )
    .bind(
      logId,
      userId,
      report.folder_id,
      logStatus,
      errorMessage || (report.success ? "Sync completed successfully" : null),
      report.duration_ms,
      report.files_transferred,
      report.files_deleted
    )
    .run();

  // Insert conflicts
  for (const conflict of report.conflicts) {
    const conflictId = generateId();
    await db
      .prepare(
        "INSERT INTO conflicts (id, user_id, folder_id, file_path, local_version, remote_version) VALUES (?1, ?2, ?3, ?4, ?5, ?6)"
      )
      .bind(
        conflictId,
        userId,
        report.folder_id,
        conflict.file_path,
        conflict.local_version || null,
        conflict.remote_version || null
      )
      .run();
  }

  return c.json({ ok: true });
});

// GET /sync/status/:folder_id - Get sync status for a folder
sync.get("/status/:folder_id", async (c) => {
  const userId = c.get("user").sub;
  const folderId = c.req.param("folder_id");

  const folder = await c.env.DB.prepare(
    "SELECT id, status, last_sync_at, needs_resync FROM folders WHERE id = ?1 AND user_id = ?2"
  )
    .bind(folderId, userId)
    .first();

  if (!folder) {
    return c.json({ error: "Folder not found" }, 404);
  }

  return c.json({ folder });
});

// GET /sync/updates?since=<ISO timestamp> - Check which folders have remote updates
sync.get("/updates", async (c) => {
  const userId = c.get("user").sub;
  const since = c.req.query("since");
  const db = c.env.DB;

  if (!since) {
    return c.json({ error: "Missing 'since' query parameter" }, 400);
  }

  // Find folders updated after the given timestamp (by another device)
  const results = await db
    .prepare(
      "SELECT id, updated_at FROM folders WHERE user_id = ?1 AND updated_at > ?2 AND is_enabled = 1"
    )
    .bind(userId, since)
    .all<{ id: string; updated_at: string }>();

  return c.json({
    updated_folders: results.results.map((r) => ({
      id: r.id,
      updated_at: r.updated_at,
    })),
  });
});

export default sync;
