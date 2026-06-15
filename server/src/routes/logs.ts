import { Hono } from "hono";
import type { Env } from "../env";
import { authMiddleware, type AuthContext } from "../middleware/auth";

const logs = new Hono<{ Bindings: Env; Variables: AuthContext }>();

// All log routes require authentication
logs.use("*", authMiddleware);

// GET /logs - List sync logs
logs.get("/", async (c) => {
  const userId = c.get("user").sub;
  const folderId = c.req.query("folder_id");
  const limit = Math.min(parseInt(c.req.query("limit") || "50"), 200);

  let query =
    "SELECT id, user_id, folder_id, timestamp, action, status, message, duration_ms, files_transferred, files_deleted FROM sync_logs WHERE user_id = ?1";

  const params: (string | number)[] = [userId];
  let paramIdx = 2;

  if (folderId) {
    query += ` AND folder_id = ?${paramIdx++}`;
    params.push(folderId);
  }

  query += ` ORDER BY timestamp DESC LIMIT ?${paramIdx}`;
  params.push(limit);

  const rows = await c.env.DB.prepare(query).bind(...params).all();

  return c.json({ logs: rows.results });
});

export default logs;
