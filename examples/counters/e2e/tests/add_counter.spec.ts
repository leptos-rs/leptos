import { test, expect } from "@playwright/test";
import { CountersPage } from "./fixtures/counters_page";

test.describe("Add Counter", () => {
  test("should increase the number of counters", async ({ page }) => {
    const ui = new CountersPage(page);
    await ui.goto();

    await ui.addCounter();
    await ui.addCounter();
    await ui.addCounter();

    await expect(ui.counters).toHaveText("3");
  });
});
