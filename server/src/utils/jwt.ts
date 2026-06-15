import { sign, verify } from "hono/jwt";
import type { Env } from "../env";

export interface AccessTokenPayload {
  sub: string; // user_id
  email: string;
  iat: number;
  exp: number;
}

export interface RefreshTokenRecord {
  id: string;
  token_hash: string;
  expires_at: string;
}

const ACCESS_TOKEN_TTL = 15 * 60; // 15 minutes
const REFRESH_TOKEN_TTL = 30 * 24 * 60 * 60; // 30 days

export async function createAccessToken(
  userId: string,
  email: string,
  secret: string
): Promise<string> {
  const now = Math.floor(Date.now() / 1000);
  const payload = {
    sub: userId,
    email,
    iat: now,
    exp: now + ACCESS_TOKEN_TTL,
  };
  return sign(payload, secret, "HS256");
}

export async function createRefreshToken(
  userId: string,
  db: D1Database
): Promise<{ token: string; record: RefreshTokenRecord }> {
  // Generate a random refresh token
  const array = new Uint8Array(32);
  crypto.getRandomValues(array);
  const token = Array.from(array, (b) => b.toString(16).padStart(2, "0")).join("");

  // Hash for storage
  const tokenHash = await hashToken(token);

  const id = crypto.randomUUID();
  const expiresAt = new Date(Date.now() + REFRESH_TOKEN_TTL * 1000).toISOString();

  // Store in DB
  await db
    .prepare("INSERT INTO refresh_tokens (id, user_id, token_hash, expires_at) VALUES (?1, ?2, ?3, ?4)")
    .bind(id, userId, tokenHash, expiresAt)
    .run();

  return { token, record: { id, token_hash: tokenHash, expires_at: expiresAt } };
}

export async function verifyAccessToken(
  token: string,
  secret: string
): Promise<AccessTokenPayload> {
  const payload = await verify(token, secret, "HS256");
  return payload as unknown as AccessTokenPayload;
}

export async function hashToken(token: string): Promise<string> {
  const encoder = new TextEncoder();
  const data = encoder.encode(token);
  const hashBuffer = await crypto.subtle.digest("SHA-256", data);
  return Array.from(new Uint8Array(hashBuffer), (b) => b.toString(16).padStart(2, "0")).join("");
}

export async function validateRefreshToken(
  token: string,
  db: D1Database
): Promise<{ userId: string; tokenId: string } | null> {
  const tokenHash = await hashToken(token);

  const row = await db
    .prepare(
      "SELECT rt.id, rt.user_id, rt.revoked, rt.expires_at FROM refresh_tokens rt WHERE rt.token_hash = ?1"
    )
    .bind(tokenHash)
    .first<{ id: string; user_id: string; revoked: number; expires_at: string }>();

  if (!row) return null;
  if (row.revoked) return null;
  if (new Date(row.expires_at) < new Date()) return null;

  return { userId: row.user_id, tokenId: row.id };
}

export async function revokeRefreshToken(tokenId: string, db: D1Database): Promise<void> {
  await db.prepare("UPDATE refresh_tokens SET revoked = 1 WHERE id = ?1").bind(tokenId).run();
}

export async function revokeAllUserTokens(userId: string, db: D1Database): Promise<void> {
  await db.prepare("UPDATE refresh_tokens SET revoked = 1 WHERE user_id = ?1").bind(userId).run();
}
