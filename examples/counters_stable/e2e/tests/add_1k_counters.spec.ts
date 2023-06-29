import { test, expect } from "@playwright/test";
import { CountersPage } from "./counters_page";

test.describe("Add 1000 Counters", () => {
  test("should increment the total count by 1K", async ({ page }) => {
    const ui = new CountersPage(page);

    await Promise.all([
      await ui.goto(),
      await ui.addOneThousandCountersButton.waitFor(),
    ]);

    await ui.addOneThousandCounters();
    await ui.addOneThousandCounters();
    await ui.addOneThousandCounters();

    await expect(ui.total).toHaveText("0");
    await expect(ui.counters).toHaveText("3000");
  });
});
