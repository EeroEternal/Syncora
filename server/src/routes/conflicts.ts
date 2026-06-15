import { Hono } from "hono";
import { z } from "zod";
import { zValidator } from "@hono/zod-validator";
import type { Env } from "../env";
import { authMiddleware, type AuthContext } from "../middleware/auth";

const conflicts = new Hono<{ Bindings: Env; Variables: AuthContext }>();

// All conflict routes require authentication
conflicts.use("*", authMiddleware);

// GET /conflicts - List user's conflicts
conflicts.get("/", async (c) => {
  const userId = c.get("user").sub;
  const resolved = c.req.query("resolved");

  let query =
    "SELECT id, user_id, folder_id, file_path, local_version, remote_version, detected_at, resolved, resolution FROM conflicts WHERE user_id = ?1";

  const params: (string | number)[] = [userId];

  if (resolved === "true") {
    query += " AND resolved = 1";
  } else if (resolved === "false") {
    query += " AND resolved = 0";
  }

  query += " ORDER BY detected_at DESC";

  const rows = await c.env.DB.prepare(query).bind(...params).all();

  return c.json({ conflicts: rows.results });
});

// PATCH /conflicts/:id - Resolve a conflict
const resolveSchema = z.object({
  resolution: z.enum(["keep_local", "keep_remote", "keep_both"]),
});

conflicts.patch("/:id", zValidator("json", resolveSchema), async (c) => {
  const userId = c.get("user").sub;
  const conflictId = c.req.param("id");
  const { resolution } = c.req.valid("json");
  const db = c.env.DB;

  const existing = await db
    .prepare("SELECT id FROM conflicts WHERE id = ?1 AND user_id = ?2")
    .bind(conflictId, userId)
    .first();

  if (!existing) {
    return c.json({ error: "Conflict not found" }, 404);
  }

  await db
    .prepare("UPDATE conflicts SET resolved = 1, resolution = ?1 WHERE id = ?2")
    .bind(resolution, conflictId)
    .run();

  return c.json({ ok: true });
});

export default conflicts;
