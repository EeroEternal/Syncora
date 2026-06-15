import { createMiddleware } from "hono/factory";
import type { Env } from "../env";
import { verifyAccessToken, type AccessTokenPayload } from "../utils/jwt";

export type AuthContext = {
  user: AccessTokenPayload;
};

export const authMiddleware = createMiddleware<{
  Bindings: Env;
  Variables: AuthContext;
}>(async (c, next) => {
  const authHeader = c.req.header("Authorization");

  if (!authHeader || !authHeader.startsWith("Bearer ")) {
    return c.json({ error: "Missing or invalid Authorization header" }, 401);
  }

  const token = authHeader.slice(7);

  try {
    const payload = await verifyAccessToken(token, c.env.JWT_SECRET);
    c.set("user", payload);
    await next();
  } catch (err: any) {
    if (err?.message?.includes("expired")) {
      return c.json({ error: "Token expired" }, 401);
    }
    return c.json({ error: "Invalid token" }, 401);
  }
});
