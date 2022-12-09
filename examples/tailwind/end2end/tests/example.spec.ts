import { test, expect } from "@playwright/test";

test("homepage has title and links to intro page", async ({ page }) => {
  await page.goto("http://localhost:3000/");

  await expect(page).toHaveTitle("Cargo Leptos");

  await expect(page.locator("h1")).toHaveText("Hi from your Leptos WASM!");
});
