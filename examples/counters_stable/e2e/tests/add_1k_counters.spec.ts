import { test, expect } from "@playwright/test";
import { CountersPage } from "./counters_page";

test.describe("Add 1000 Counters", () => {
  test.skip("should increment the total count by 1K", async ({ page }) => {
    const ui = new CountersPage(page);
    await ui.goto();

    await ui.addOneThousandCounters();
    await expect(ui.total).toHaveText("0");
    await expect(ui.counters).toHaveText("1000");

    await ui.addOneThousandCounters();
    await expect(ui.total).toHaveText("0");
    await expect(ui.counters).toHaveText("2000");

    await ui.addOneThousandCounters();
    await expect(ui.total).toHaveText("0");
    await expect(ui.counters).toHaveText("3000");
  });
});
