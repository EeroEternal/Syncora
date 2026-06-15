import { cors } from "hono/cors";

export const corsMiddleware = cors({
  origin: [
    "http://localhost:3000",
    "http://localhost:1420",
    "http://localhost:8787",
    "tauri://localhost",
    "http://tauri.localhost",
  ],
  allowMethods: ["GET", "POST", "PATCH", "DELETE", "OPTIONS"],
  allowHeaders: ["Authorization", "Content-Type"],
  maxAge: 86400,
});
