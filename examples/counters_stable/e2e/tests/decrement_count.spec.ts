import { test, expect } from "@playwright/test";
import { CountersPage } from "./counters-page";

test.describe("Decrement Count", () => {
  test("should decrement the total count", async ({ page }) => {
    const counters = new CountersPage(page);
    await counters.goto();
    await counters.addCounter();

    await counters.decrementCount();
    await counters.decrementCount();
    await counters.decrementCount();

    await expect(counters.total).toHaveText("-3");
    await expect(counters.counters).toHaveText("1");
  });
});
