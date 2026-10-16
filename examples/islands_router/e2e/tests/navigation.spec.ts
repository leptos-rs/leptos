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

test("navigates from Home to About, increments Counter", async ({ page }) => {
  const errors = collectErrors(page);

  await page.goto("/");
  await expect(page).toHaveTitle("Home | My Contacts");
  await expect(page.locator("input[type=search]")).toBeVisible();

  await page.click('nav a[href="/about"]');
  await expect(page).toHaveURL(/\/about$/);
  await expect(page.getByRole("heading", { name: "About" })).toBeVisible();
  await expect(page).toHaveTitle("About | My Contacts");
  await expect(page.locator("input[type=search]")).not.toBeAttached();

  await expect(page.getByText("Enter a search to begin viewing contacts.")).not.toBeVisible();
  await expect(page.locator("button[class=counter]")).toHaveText("Click Me: 0");
  await page.click('button[class=counter]');
  await expect(page.locator("button[class=counter]")).toHaveText("Click Me: 1");

  expect(errors).toEqual([]);
});

test("navigates from Home to About (redundantly), increments Counter", async ({ page }) => {
  const errors = collectErrors(page);

  await page.goto("/");
  await expect(page).toHaveTitle("Home | My Contacts");
  await expect(page.locator("input[type=search]")).toBeVisible();

  await page.click('nav a[href="/about"]');
  await expect(page).toHaveURL(/\/about$/);
  await expect(page.getByRole("heading", { name: "About" })).toBeVisible();
  await expect(page).toHaveTitle("About | My Contacts");
  await expect(page.locator("input[type=search]")).not.toBeAttached();

  await page.click('nav a[href="/about"]'); // 2
  await page.click('nav a[href="/about"]'); // 3
  await page.click('nav a[href="/about"]'); // 4

  await expect(page.locator("button[class=counter]")).toBeAttached();
  await expect(page.getByText("Enter a search to begin viewing contacts.")).not.toBeVisible();
  await expect(page.locator("button[class=counter]")).toHaveText("Click Me: 0");
  await page.click('button[class=counter]');
  await expect(page.locator("button[class=counter]")).toHaveText("Click Me: 1");

  expect(errors).toEqual([]);
});

test("an island that leaves the page is disposed", async ({ page }) => {
  const errors = collectErrors(page);
  const logs: string[] = [];
  page.on("console", (msg) => logs.push(msg.text()));

  await page.goto("/about");
  await page.click("button[class=counter]");
  await expect(page.locator("button[class=counter]")).toHaveText("Click Me: 1");

  await page.click('nav a[href="/"]');
  await expect(page.locator("button[class=counter]")).not.toBeAttached();
  await expect.poll(() => logs).toContain("Counter disposed");

  await page.click('nav a[href="/about"]');
  await expect(page.locator("button[class=counter]")).toHaveText("Click Me: 0");
  await page.click("button[class=counter]");
  await expect(page.locator("button[class=counter]")).toHaveText("Click Me: 1");

  expect(errors).toEqual([]);
});
