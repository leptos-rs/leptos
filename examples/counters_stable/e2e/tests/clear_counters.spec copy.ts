import { test, expect } from "@playwright/test";
import { CountersPage } from "./counters-page";

test.describe("Clear Counters", () => {
  test("should reset the counts", async ({ page }) => {
    const counters = new CountersPage(page);
    await counters.goto();
    await counters.addCounter();
    await counters.addCounter();
    await counters.addCounter();

    await counters.clearCounters();

    await expect(counters.total).toHaveText("0");
    await expect(counters.counters).toHaveText("0");
  });
});
