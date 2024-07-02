import { test, expect } from "@playwright/test";
import { HomePage } from "./fixtures/home_page";

test.describe("Clear Number", () => {
  test("should see the error message", async ({ page }) => {
    const ui = new HomePage(page);
    await ui.goto();

    await ui.clearInput();

    await expect(ui.errorMessage).toHaveText("Not an integer! Errors: ");
  });
  test("should see the error list", async ({ page }) => {
    const ui = new HomePage(page);
    await ui.goto();

    await ui.clearInput();

    await expect(ui.errorList).toHaveText(
      "cannot parse integer from empty string"
    );
  });
});
