import { test, expect } from "@playwright/test";

test("homepage has title 'Leptos + Tailwindcss'", async ({ page }) => {
  await page.goto("http://localhost:3000/");

  await expect(page).toHaveTitle("Leptos + Tailwindcss");
});
