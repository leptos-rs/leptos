import { test, expect } from "@playwright/test";
import { CountersPage } from "./fixtures/counters_page";

test.describe("Remove Counter", () => {
  test("should decrement the number of counters", async ({ page }) => {
    const ui = new CountersPage(page);
    await ui.goto();

    await ui.addCounter();
    await ui.addCounter();
    await ui.addCounter();

    await ui.removeCounter(1);

    await expect(ui.counters).toHaveText("2");
  });
});
