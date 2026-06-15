import { Hono } from "hono";
import { z } from "zod";
import { zValidator } from "@hono/zod-validator";
import type { Env } from "../env";
import { hashPassword, verifyPassword } from "../utils/password";
import {
  createAccessToken,
  createRefreshToken,
  validateRefreshToken,
  revokeRefreshToken,
  verifyAccessToken,
} from "../utils/jwt";
import { generateUserId } from "../utils/id";
import { authMiddleware, type AuthContext } from "../middleware/auth";

const auth = new Hono<{ Bindings: Env; Variables: AuthContext }>();

// Validation schemas
const registerSchema = z.object({
  email: z.string().email(),
  password: z.string().min(8).max(128),
  display_name: z.string().min(1).max(100).optional(),
});

const loginSchema = z.object({
  email: z.string().email(),
  password: z.string(),
});

const refreshSchema = z.object({
  refresh_token: z.string(),
});

// POST /auth/register
auth.post("/register", zValidator("json", registerSchema), async (c) => {
  const { email, password, display_name } = c.req.valid("json");
  const db = c.env.DB;

  // Check if email already exists
  const existing = await db
    .prepare("SELECT id FROM users WHERE email = ?1")
    .bind(email)
    .first();

  if (existing) {
    return c.json({ error: "Email already registered" }, 409);
  }

  // Create user
  const userId = generateUserId();
  const passwordHash = await hashPassword(password);

  await db
    .prepare(
      "INSERT INTO users (id, email, password_hash, display_name, r2_prefix) VALUES (?1, ?2, ?3, ?4, ?5)"
    )
    .bind(userId, email, passwordHash, display_name || null, userId)
    .run();

  // Generate tokens
  const accessToken = await createAccessToken(userId, email, c.env.JWT_SECRET);
  const { token: refreshToken } = await createRefreshToken(userId, db);

  return c.json(
    {
      user: {
        id: userId,
        email,
        display_name: display_name || null,
      },
      access_token: accessToken,
      refresh_token: refreshToken,
    },
    201
  );
});

// POST /auth/login
auth.post("/login", zValidator("json", loginSchema), async (c) => {
  const { email, password } = c.req.valid("json");
  const db = c.env.DB;

  const user = await db
    .prepare("SELECT id, email, password_hash, display_name FROM users WHERE email = ?1")
    .bind(email)
    .first<{ id: string; email: string; password_hash: string; display_name: string | null }>();

  if (!user) {
    return c.json({ error: "Invalid credentials" }, 401);
  }

  const valid = await verifyPassword(password, user.password_hash);
  if (!valid) {
    return c.json({ error: "Invalid credentials" }, 401);
  }

  // Generate tokens
  const accessToken = await createAccessToken(user.id, user.email, c.env.JWT_SECRET);
  const { token: refreshToken } = await createRefreshToken(user.id, db);

  return c.json({
    user: {
      id: user.id,
      email: user.email,
      display_name: user.display_name,
    },
    access_token: accessToken,
    refresh_token: refreshToken,
  });
});

// POST /auth/refresh
auth.post("/refresh", zValidator("json", refreshSchema), async (c) => {
  const { refresh_token } = c.req.valid("json");
  const db = c.env.DB;

  const result = await validateRefreshToken(refresh_token, db);
  if (!result) {
    return c.json({ error: "Invalid or expired refresh token" }, 401);
  }

  // Revoke old token
  await revokeRefreshToken(result.tokenId, db);

  // Get user info
  const user = await db
    .prepare("SELECT id, email FROM users WHERE id = ?1")
    .bind(result.userId)
    .first<{ id: string; email: string }>();

  if (!user) {
    return c.json({ error: "User not found" }, 401);
  }

  // Generate new token pair
  const accessToken = await createAccessToken(user.id, user.email, c.env.JWT_SECRET);
  const { token: newRefreshToken } = await createRefreshToken(user.id, db);

  return c.json({
    access_token: accessToken,
    refresh_token: newRefreshToken,
  });
});

// POST /auth/logout
auth.post("/logout", authMiddleware, async (c) => {
  const refreshToken = c.req.header("X-Refresh-Token");
  if (refreshToken) {
    const result = await validateRefreshToken(refreshToken, c.env.DB);
    if (result) {
      await revokeRefreshToken(result.tokenId, c.env.DB);
    }
  }
  return c.json({ ok: true });
});

// GET /auth/me
auth.get("/me", authMiddleware, async (c) => {
  const userPayload = c.get("user");
  const db = c.env.DB;

  const user = await db
    .prepare("SELECT id, email, display_name, created_at FROM users WHERE id = ?1")
    .bind(userPayload.sub)
    .first<{ id: string; email: string; display_name: string | null; created_at: string }>();

  if (!user) {
    return c.json({ error: "User not found" }, 404);
  }

  return c.json({
    user: {
      id: user.id,
      email: user.email,
      display_name: user.display_name,
      created_at: user.created_at,
    },
  });
});

export default auth;
