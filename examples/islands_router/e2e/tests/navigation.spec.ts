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

test("navigates from Home to About", async ({ page }) => {
  const errors = collectErrors(page);

  await page.goto("/");
  await expect(page).toHaveTitle("Home | My Contacts");
  await expect(page.locator("input[type=search]")).toBeVisible();

  await page.click('nav a[href="/about"]');
  await expect(page).toHaveURL(/\/about$/);
  await expect(page.getByRole("heading", { name: "About" })).toBeVisible();
  await expect(page).toHaveTitle("About | My Contacts");
  await expect(page.locator("input[type=search]")).not.toBeAttached();

  expect(errors).toEqual([]);
});
