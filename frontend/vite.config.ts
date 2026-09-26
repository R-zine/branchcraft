import react from "@vitejs/plugin-react";
import { defineConfig } from "vitest/config";

export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    strictPort: true,
    watch: {
      ignored: ["**/src-tauri/**"],
    },
  },
  envPrefix: ["VITE_", "TAURI_ENV_*"],
  build: {
    target: "es2022",
    sourcemap: false,
    // Monaco is isolated from startup and currently emits a 2.7 MB lazy chunk.
    chunkSizeWarningLimit: 3000,
  },
  test: {
    environment: "jsdom",
    setupFiles: ["./src/test/setup.ts"],
  },
});

