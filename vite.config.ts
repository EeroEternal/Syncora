import { defineConfig } from "vite";
import solidPlugin from "vite-plugin-solid";
import tailwindcss from "@tailwindcss/vite";
import { resolve } from "path";

export default defineConfig({
  plugins: [tailwindcss(), solidPlugin()],
  resolve: {
    alias: {
      "~": resolve(__dirname, "./src"),
    },
  },
  server: {
    port: 3000,
    strictPort: true,
  },
  build: {
    target: "esnext",
  },
  clearScreen: false,
});
