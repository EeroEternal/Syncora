import { Hono } from "hono";
import type { Env } from "./env";
import type { AuthContext } from "./middleware/auth";
import { corsMiddleware } from "./middleware/cors";
import authRoutes from "./routes/auth";
import foldersRoutes from "./routes/folders";
import syncRoutes from "./routes/sync";
import conflictsRoutes from "./routes/conflicts";
import logsRoutes from "./routes/logs";

const app = new Hono<{ Bindings: Env; Variables: AuthContext }>();

// Global middleware
app.use("*", corsMiddleware);

// Health check
app.get("/api/v1/health", (c) => c.json({ status: "ok", version: "0.1.0" }));

// Routes
app.route("/api/v1/auth", authRoutes);
app.route("/api/v1/folders", foldersRoutes);
app.route("/api/v1/sync", syncRoutes);
app.route("/api/v1/conflicts", conflictsRoutes);
app.route("/api/v1/logs", logsRoutes);

// 404 handler
app.notFound((c) => c.json({ error: "Not found" }, 404));

// Error handler
app.onError((err, c) => {
  console.error("Unhandled error:", err);
  return c.json({ error: "Internal server error" }, 500);
});

export default app;
