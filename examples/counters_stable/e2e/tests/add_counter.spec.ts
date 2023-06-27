import { test, expect } from "@playwright/test";
import { CountersPage } from "./counters_page";

test.describe("Add Counter", () => {
  test("should increment the total count", async ({ page }) => {
    const ui = new CountersPage(page);
    await ui.goto();

    await ui.addCounter();
    await ui.addCounter();
    await ui.addCounter();

    await expect(ui.total).toHaveText("0");
    await expect(ui.counters).toHaveText("3");
  });
});
