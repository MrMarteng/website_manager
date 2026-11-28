import { defineConfig } from "vitest/config";

// Tauri serves the build from ui/dist (release) or this dev server (tauri dev).
export default defineConfig({
  root: "ui",
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: "127.0.0.1",
  },
  build: {
    outDir: "dist",
    emptyOutDir: true,
    // Targets of the system web views (Tauri guide).
    target: ["es2021", "chrome105", "safari15"],
    // No inline scripts or styles: the CSP allows 'self' only.
    assetsInlineLimit: 0,
    modulePreload: { polyfill: false },
    sourcemap: false,
  },
  test: {
    root: ".",
    environment: "happy-dom",
    include: ["ui/src/**/*.test.ts"],
  },
});
