import { defineConfig } from "vitest/config";
import solid from "vite-plugin-solid";

// Tauri expects a fixed port and no clearing of its own log output.
export default defineConfig({
  plugins: [solid()],
  clearScreen: false,
  server: { port: 1420, strictPort: true, watch: { ignored: ["**/src-tauri/**"] } },
  build: { target: "safari16", outDir: "dist" },
  // globals lets @solidjs/testing-library register its afterEach cleanup.
  test: { environment: "jsdom", globals: true },
  resolve: { conditions: ["development", "browser"] },
});
