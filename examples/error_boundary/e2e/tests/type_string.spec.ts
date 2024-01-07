import { test, expect } from "@playwright/test";
import { HomePage } from "./fixtures/home_page";

test.describe("Type String", () => {
  test("should see the error message", async ({ page }) => {
    const ui = new HomePage(page);
    await ui.goto();

    await ui.enterValue("leptos");

    await expect(ui.errorMessage).toHaveText("Not a number! Errors: ");
  });
  test("should see the error list", async ({ page }) => {
    const ui = new HomePage(page);
    await ui.goto();

    await ui.enterValue("leptos");

    await expect(ui.errorList).toHaveText(
      "invalid digit found in string"
    );
  });
});
