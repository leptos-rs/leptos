import { test, expect } from "@playwright/test";
import { CountersPage } from "./fixtures/counters_page";

test.describe("Add 1000 Counters", () => {
  test("should increase the number of counters", async ({ page }) => {
    const ui = new CountersPage(page);

    await Promise.all([
      await ui.goto(),
      await ui.addOneThousandCountersButton.waitFor(),
    ]);

    await ui.addOneThousandCounters();
    await ui.addOneThousandCounters();
    await ui.addOneThousandCounters();

    await expect(ui.counters).toHaveText("3000");
  });
});
