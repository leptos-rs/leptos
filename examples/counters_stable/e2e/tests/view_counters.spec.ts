import { test, expect } from "@playwright/test";
import { CountersPage } from "./counters_page";

test.describe("View Counters", () => {
  test("should_see_the_title", async ({ page }) => {
    const ui = new CountersPage(page);
    await ui.goto();

    await expect(page).toHaveTitle("Counters (Stable)");
  });

  test("should see the initial counts", async ({ page }) => {
    const counters = new CountersPage(page);
    await counters.goto();

    await expect(counters.total).toHaveText("0");
    await expect(counters.counters).toHaveText("0");
  });
});
