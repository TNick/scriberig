/** @file Configures the full browser suite for the reference application. */

import { defineConfig, devices } from "@playwright/test";

/** Runs the application E2E suite against the managed local services. */
export default defineConfig({
  testDir: "./specs",
  globalSetup: "./global-setup.ts",
  globalTeardown: "./global-teardown.ts",
  outputDir: "../../output/playwright/test-results",
  reporter: [["list"], ["html", { outputFolder: "../../output/playwright/report", open: "never" }]],
  timeout: 30_000,
  expect: { timeout: 5_000 },
  fullyParallel: false,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 1 : 0,
  workers: 1,
  use: {
    baseURL: `http://127.0.0.1:${process.env.SCRIBE_WEB_PORT ?? "5173"}`,
    channel: process.env.PLAYWRIGHT_CHANNEL ?? "chrome",
    trace: "on-first-retry",
    screenshot: "only-on-failure",
    video: "off",
    ...devices["Desktop Chrome"],
  },
});
