import { test, expect } from "@playwright/test";
import { HomePage } from "./fixtures/home_page";

test.describe("Open App", () => {
  test("should see the page title", async ({ page }) => {
    const ui = new HomePage(page);
    await ui.goto();

    await expect(ui.pageTitle).toHaveText("Error Handling");
  });
});
