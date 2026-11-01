import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// KUKIA CODE — Vite config
export default defineConfig({
  root: "app",
  plugins: [react()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    watch: { ignored: ["**/src-tauri/**", "**/core/**"] },
  },
  build: {
    outDir: "../dist",
    emptyOutDir: true,
    target: "chrome110",
  },
});
