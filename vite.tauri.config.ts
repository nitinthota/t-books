import path from "node:path";
import { defineConfig } from "vite";
import viteReact from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";

export default defineConfig({
  root: path.resolve(import.meta.dirname, "tauri-web"),
  publicDir: path.resolve(import.meta.dirname, "public"),
  plugins: [tailwindcss(), viteReact()],
  resolve: {
    tsconfigPaths: true,
    alias: {
      "@": path.resolve(import.meta.dirname, "src"),
    },
  },
  build: {
    outDir: path.resolve(import.meta.dirname, "dist/client"),
    emptyOutDir: true,
  },
});
