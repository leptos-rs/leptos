import { test, expect } from "@playwright/test";
import { CountersPage } from "./fixtures/counters_page";

test.describe("Clear Counters", () => {
  test("should reset the counts", async ({ page }) => {
    const ui = new CountersPage(page);
    await ui.goto();

    await ui.addCounter();
    await ui.addCounter();
    await ui.addCounter();

    await ui.clearCounters();

    await expect(ui.total).toHaveText("0");
    await expect(ui.counters).toHaveText("0");
  });
});
