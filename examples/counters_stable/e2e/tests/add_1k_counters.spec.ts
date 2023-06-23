import { test, expect } from "@playwright/test";
import { CountersPage } from "./counters-page";

test.describe("Add 1000 Counters", () => {
  test("should increment the total count by 1K", async ({ page }) => {
    const counters = new CountersPage(page);
    await counters.goto();

    await counters.addOneThousandCounters();
    await counters.addOneThousandCounters();
    await counters.addOneThousandCounters();

    await expect(counters.total).toHaveText("0");
    await expect(counters.counters).toHaveText("3000");
  });
});
