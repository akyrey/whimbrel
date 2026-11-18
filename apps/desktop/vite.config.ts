/// <reference types="vitest/config" />
import process from "node:process";

import tailwindcss from "@tailwindcss/vite";
import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

const host = process.env.TAURI_DEV_HOST;

// https://v2.tauri.app/start/frontend/vite/
export default defineConfig({
  plugins: [react(), tailwindcss()],
  resolve: {
    // @roost/* ship TypeScript source with these as peers: one copy each, also when Roost is
    // linked from a local checkout (scripts/link-roost.sh).
    dedupe: [
      "react",
      "react-dom",
      "zustand",
      "@tanstack/react-virtual",
      "codemirror",
      "@codemirror/state",
      "@codemirror/view",
      "@codemirror/language",
      "@codemirror/lang-json",
      "@codemirror/lang-xml",
      "@lezer/highlight",
    ],
  },
  // Keep Rust errors visible in `tauri dev`.
  clearScreen: false,
  server: {
    // Tauri expects a fixed port.
    port: 1430,
    strictPort: true,
    host: host || false,
    hmr: host ? { protocol: "ws", host, port: 1431 } : undefined,
    watch: { ignored: ["**/src-tauri/**"] },
  },
  // Loaded from disk by the desktop shell, so bundle size matters less than on the web.
  build: { chunkSizeWarningLimit: 1024 },
  test: {
    environment: "jsdom",
    globals: true,
    setupFiles: ["./src/test/setup.ts"],
    // Roost's packages are TypeScript source: compile them like the app's own.
    server: { deps: { inline: [/@roost\//] } },
    include: ["src/**/*.test.{ts,tsx}", "scripts/**/*.test.mjs"],
    coverage: {
      provider: "v8",
      include: ["src/**/*.{ts,tsx}"],
      exclude: ["src/ipc/generated/**", "src/main.tsx", "src/test/**", "src/**/*.test.{ts,tsx}"],
      thresholds: { lines: 80, functions: 80, branches: 80, statements: 80 },
    },
  },
});
