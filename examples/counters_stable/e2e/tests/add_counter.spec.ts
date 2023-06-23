import { test, expect } from "@playwright/test";
import { CountersPage } from "./counters-page";

test.describe("Add Counter", () => {
  test("should increment the count", async ({ page }) => {
    const counters = new CountersPage(page);
    await counters.goto();

    await counters.addCounter();
    await counters.addCounter();
    await counters.addCounter();

    await expect(counters.total).toHaveText("0");
    await expect(counters.counters).toHaveText("3");
  });
});
