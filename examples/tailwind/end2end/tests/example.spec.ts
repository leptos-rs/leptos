import { test, expect } from "@playwright/test";

test("should see the welcome message", async ({ page }) => {
  await page.goto("http://localhost:3000/");

  await expect(page.locator("h2")).toHaveText("Welcome to Leptos with Tailwind");
});
