import { test, expect } from "@playwright/test";

test("homepage has title 'Leptos + Tailwindcss'", async ({ page }) => {
  await page.goto("http://localhost:8080/");

  await expect(page).toHaveTitle("Leptos + Tailwindcss");
});
