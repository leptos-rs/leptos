import { test, expect } from "@playwright/test";
import { CountersPage } from "./counters_page";

test.describe("Decrement Count", () => {
  test("should decrement the total count", async ({ page }) => {
    const ui = new CountersPage(page);
    await ui.goto();
    await ui.addCounter();

    await ui.decrementCount();
    await ui.decrementCount();
    await ui.decrementCount();

    await expect(ui.total).toHaveText("-3");
    await expect(ui.counters).toHaveText("1");
  });
});
