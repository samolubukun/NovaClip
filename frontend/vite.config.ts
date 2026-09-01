import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
import path from "path";

export default defineConfig({
  plugins: [react(), tailwindcss()],
  resolve: {
    alias: { "@": path.resolve(__dirname, "./src") },
  },
  server: {
    port: 5173,
    strictPort: true,
    proxy: {
      "/tasks": "http://localhost:8000",
      "/media": "http://localhost:8000",
      "/health": "http://localhost:8000",
      "/studio": "http://localhost:8000",
      "/mcp": "http://localhost:8000",
    },
  },
  // Tauri expects the dev server to be available quickly; disable HMR overlay clash
  clearScreen: false,
  envPrefix: ["VITE_"],
});
