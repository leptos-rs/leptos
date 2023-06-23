import { test, expect } from "@playwright/test";
import { CountersPage } from "./counters-page";

test.describe("Increment Count", () => {
  test("should increment the total count", async ({ page }) => {
    const counters = new CountersPage(page);
    await counters.goto();
    await counters.addCounter();

    await counters.incrementCount();
    await counters.incrementCount();
    await counters.incrementCount();

    await expect(counters.total).toHaveText("3");
    await expect(counters.counters).toHaveText("1");
  });
});
