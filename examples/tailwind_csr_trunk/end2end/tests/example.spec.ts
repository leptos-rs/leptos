import { test, expect } from "@playwright/test";

test("homepage has title and links to intro page", async ({ page }) => {
  await page.goto("http://localhost:8080/");

  await expect(page).toHaveTitle("Leptos • Counter with Tailwind");

  await expect(page.locator("h2")).toHaveText("Welcome to Leptos with Tailwind");
});
