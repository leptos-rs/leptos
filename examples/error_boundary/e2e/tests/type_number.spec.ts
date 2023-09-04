import { test, expect } from "@playwright/test";
import { HomePage } from "./fixtures/home_page";

test.describe("Type Number", () => {
  test("should see the typed number", async ({ page }) => {
    const ui = new HomePage(page);
    await ui.goto();

    await ui.enterNumber("7");

    await expect(ui.successMessage).toHaveText("You entered 7");
  });
});
