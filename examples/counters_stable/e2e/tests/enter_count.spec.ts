import { test, expect } from "@playwright/test";
import { CountersPage } from "./fixtures/counters_page";

test.describe("Enter Count", () => {
  test("should increase the total count", async ({ page }) => {
    const ui = new CountersPage(page);
    await ui.goto();
    await ui.addCounter();

    await ui.enterCount("5");

    await expect(ui.total).toHaveText("5");
    await expect(ui.counters).toHaveText("1");
  });

  test("should decrease the total count", async ({ page }) => {
    const ui = new CountersPage(page);
    await ui.goto();
    await ui.addCounter();
    await ui.addCounter();
    await ui.addCounter();

    await ui.enterCount("100");
    await ui.enterCount("100", 1);
    await ui.enterCount("100", 2);
    await ui.enterCount("50", 1);

    await expect(ui.total).toHaveText("250");
  });
});
