import { test, expect, Page } from "@playwright/test";

function collectErrors(page: Page): string[] {
  const errors: string[] = [];
  page.on("console", (msg) => {
    if (msg.type() === "error") {
      errors.push(msg.text());
    }
  });
  page.on("pageerror", (err) => {
    errors.push(String(err));
  });
  return errors;
}

test.describe("islands router branch replacement", () => {
  test("navigates between plain pages (control)", async ({ page }) => {
    const errors = collectErrors(page);

    await page.goto("/about");
    await expect(page.locator("h2", { hasText: "About" })).toBeVisible();

    await page.click('nav a[href="/"]');
    await expect(page).toHaveURL(/\/$/);
    await expect(page.locator("input[type=search]")).toBeVisible();
    await expect(page.locator("h2", { hasText: "About" })).not.toBeAttached();

    expect(errors).toEqual([]);
  });

  // Regression test: replaceBranch used to skip the first node after a
  // branch marker while counting nested branches. A route view whose first
  // node renders to marker comments (e.g. a leptos_meta tag) made it close
  // the branch range early, so the old page's HTML was never removed.
  test("replaces a route view that starts with marker comments", async ({
    page,
  }) => {
    const errors = collectErrors(page);

    await page.goto("/marker-first");
    await expect(page.locator("#marker-first-heading")).toBeVisible();

    await page.click('nav a[href="/about"]');
    await expect(page).toHaveURL(/\/about$/);
    await expect(page.locator("h2", { hasText: "About" })).toBeVisible();
    await expect(page.locator("#marker-first-heading")).not.toBeAttached();

    expect(errors).toEqual([]);
  });

  test("navigates into a route view that starts with marker comments", async ({
    page,
  }) => {
    const errors = collectErrors(page);

    await page.goto("/about");
    await expect(page.locator("h2", { hasText: "About" })).toBeVisible();

    await page.click('nav a[href="/marker-first"]');
    await expect(page).toHaveURL(/\/marker-first$/);
    await expect(page.locator("#marker-first-heading")).toBeVisible();
    await expect(page.locator("h2", { hasText: "About" })).not.toBeAttached();

    expect(errors).toEqual([]);
  });
});
