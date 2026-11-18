import { defineConfig, devices } from "@playwright/test";

const PORT = 5189;

/** E2E runs the frontend in a browser against the mocked backend (DECISIONS D-011). */
export default defineConfig({
  testDir: "./e2e",
  fullyParallel: true,
  forbidOnly: Boolean(process.env.CI),
  retries: process.env.CI ? 1 : 0,
  reporter: process.env.CI ? [["list"], ["html", { open: "never" }]] : "list",
  use: { baseURL: `http://localhost:${PORT}`, trace: "retain-on-failure" },
  projects: [{ name: "chromium", use: { ...devices["Desktop Chrome"] } }],
  webServer: {
    command: `pnpm exec vite --port ${PORT} --strictPort`,
    url: `http://localhost:${PORT}`,
    env: { VITE_E2E: "true" },
    reuseExistingServer: !process.env.CI,
  },
});
