import { test, expect } from "@playwright/test";
import { HomePage } from "./fixtures/home_page";

test.describe("Click Down Arrow", () => {
  test("should see the negative number", async ({ page }) => {
    const ui = new HomePage(page);
    await ui.goto();

    await ui.clickDownArrow();
    await ui.clickDownArrow();
    await ui.clickDownArrow();
    await ui.clickDownArrow();
    await ui.clickDownArrow();

    await expect(ui.successMessage).toHaveText("You entered -5");
  });
});
