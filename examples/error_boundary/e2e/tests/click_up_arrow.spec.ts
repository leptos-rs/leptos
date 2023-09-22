import { test, expect } from "@playwright/test";
import { HomePage } from "./fixtures/home_page";

test.describe("Click Up Arrow", () => {
  test("should see the positive number", async ({ page }) => {
    const ui = new HomePage(page);
    await ui.goto();

    await ui.clickUpArrow();
    await ui.clickUpArrow();
    await ui.clickUpArrow();

    await expect(ui.successMessage).toHaveText("You entered 3");
  });
});
