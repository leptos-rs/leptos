import { defineConfig, devices } from "@playwright/test";

/**
 * See https://playwright.dev/docs/test-configuration.
 *
 * Run the example first (from the example root):
 *   cargo leptos serve
 * then, in this directory:
 *   npm install && npx playwright test
 */
export default defineConfig({
  testDir: "./tests",
  fullyParallel: true,
  forbidOnly: !process.env.DEV,
  retries: process.env.DEV ? 0 : 2,
  workers: 1,
  reporter: [["html", { open: "never" }], ["list"]],
  use: {
    /* The example's site-addr from Cargo.toml's [package.metadata.leptos]. */
    baseURL: "http://127.0.0.1:3009",
    trace: "on-first-retry",
  },
  projects: [
    {
      name: "chromium",
      use: { ...devices["Desktop Chrome"] },
    },
  ],
});
