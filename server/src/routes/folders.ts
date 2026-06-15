import { Hono } from "hono";
import { z } from "zod";
import { zValidator } from "@hono/zod-validator";
import type { Env } from "../env";
import { authMiddleware, type AuthContext } from "../middleware/auth";
import { generateId } from "../utils/id";

const REMOTE_PREFIX_REGEX = /^[a-zA-Z0-9][a-zA-Z0-9._-]{0,127}$/;

const folders = new Hono<{ Bindings: Env; Variables: AuthContext }>();

// All folder routes require authentication
folders.use("*", authMiddleware);

// GET /folders - List user's folders
folders.get("/", async (c) => {
  const userId = c.get("user").sub;
  const rows = await c.env.DB.prepare(
    "SELECT id, user_id, local_path_hint, remote_prefix, mode, last_sync_at, status, is_enabled, needs_resync, created_at, updated_at FROM folders WHERE user_id = ?1 ORDER BY created_at DESC"
  )
    .bind(userId)
    .all();

  return c.json({ folders: rows.results });
});

// POST /folders - Create folder
const createFolderSchema = z.object({
  local_path_hint: z.string().optional(),
  remote_prefix: z
    .string()
    .min(1)
    .max(128)
    .regex(REMOTE_PREFIX_REGEX, "Invalid remote prefix format"),
  force: z.boolean().optional(),
  connect: z.boolean().optional(),
});

folders.post("/", zValidator("json", createFolderSchema), async (c) => {
  const userId = c.get("user").sub;
  const { local_path_hint, remote_prefix, force, connect } = c.req.valid("json");
  const db = c.env.DB;

  // Check for duplicate prefix within user's folders
  const existing = await db
    .prepare("SELECT id FROM folders WHERE user_id = ?1 AND remote_prefix = ?2")
    .bind(userId, remote_prefix)
    .first<{ id: string }>();

  if (existing) {
    if (connect) {
      // Connect mode: update existing folder's local_path_hint and return it
      await db
        .prepare("UPDATE folders SET local_path_hint = ?1, updated_at = datetime('now') WHERE id = ?2 AND user_id = ?3")
        .bind(local_path_hint || null, existing.id, userId)
        .run();

      const folder = await db
        .prepare("SELECT * FROM folders WHERE id = ?1")
        .bind(existing.id)
        .first();

      return c.json({ folder });
    }

    if (force) {
      // Replace mode: delete the existing folder (and related data) before re-creating
      const oldId = existing.id;
      await db.prepare("DELETE FROM sync_logs WHERE folder_id = ?1").bind(oldId).run();
      await db.prepare("DELETE FROM conflicts WHERE folder_id = ?1").bind(oldId).run();
      await db.prepare("DELETE FROM folders WHERE id = ?1 AND user_id = ?2").bind(oldId, userId).run();
    } else {
      return c.json({ error: "Folder with this remote prefix already exists" }, 409);
    }
  }

  const id = generateId();
  await db
    .prepare(
      "INSERT INTO folders (id, user_id, local_path_hint, remote_prefix) VALUES (?1, ?2, ?3, ?4)"
    )
    .bind(id, userId, local_path_hint || null, remote_prefix)
    .run();

  const folder = await db
    .prepare("SELECT * FROM folders WHERE id = ?1")
    .bind(id)
    .first();

  return c.json({ folder }, 201);
});

// PATCH /folders/:id - Update folder
const updateFolderSchema = z.object({
  local_path_hint: z.string().optional(),
  mode: z.enum(["normal", "cloud_only"]).optional(),
  is_enabled: z.boolean().optional(),
  needs_resync: z.boolean().optional(),
});

folders.patch("/:id", zValidator("json", updateFolderSchema), async (c) => {
  const userId = c.get("user").sub;
  const folderId = c.req.param("id");
  const updates = c.req.valid("json");
  const db = c.env.DB;

  // Verify ownership
  const existing = await db
    .prepare("SELECT id FROM folders WHERE id = ?1 AND user_id = ?2")
    .bind(folderId, userId)
    .first();

  if (!existing) {
    return c.json({ error: "Folder not found" }, 404);
  }

  // Build dynamic update
  const sets: string[] = [];
  const values: (string | number)[] = [];
  let paramIdx = 1;

  if (updates.local_path_hint !== undefined) {
    sets.push(`local_path_hint = ?${paramIdx++}`);
    values.push(updates.local_path_hint);
  }
  if (updates.mode !== undefined) {
    sets.push(`mode = ?${paramIdx++}`);
    values.push(updates.mode);
  }
  if (updates.is_enabled !== undefined) {
    sets.push(`is_enabled = ?${paramIdx++}`);
    values.push(updates.is_enabled ? 1 : 0);
  }
  if (updates.needs_resync !== undefined) {
    sets.push(`needs_resync = ?${paramIdx++}`);
    values.push(updates.needs_resync ? 1 : 0);
  }

  if (sets.length > 0) {
    sets.push(`updated_at = datetime('now')`);
    values.push(folderId);
    values.push(userId);
    await db
      .prepare(`UPDATE folders SET ${sets.join(", ")} WHERE id = ?${paramIdx++} AND user_id = ?${paramIdx}`)
      .bind(...values)
      .run();
  }

  const folder = await db
    .prepare("SELECT * FROM folders WHERE id = ?1 AND user_id = ?2")
    .bind(folderId, userId)
    .first();

  return c.json({ folder });
});

// DELETE /folders/:id
folders.delete("/:id", async (c) => {
  const userId = c.get("user").sub;
  const folderId = c.req.param("id");
  const db = c.env.DB;

  const result = await db
    .prepare("DELETE FROM folders WHERE id = ?1 AND user_id = ?2")
    .bind(folderId, userId)
    .run();

  if (result.meta.changes === 0) {
    return c.json({ error: "Folder not found" }, 404);
  }

  return c.json({ ok: true });
});

export default folders;
