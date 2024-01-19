import { test, expect } from "@playwright/test";

test.describe("Test Router example", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/");
  });

  test("Starts on correct home page", async({ page }) => {  
       await expect(page.getByText("Select a contact.")).toBeVisible();
  });

  const links = [
    { label: "Bill Smith", url: "/0" },
    { label: "Tim Jones", url: "/1" },
    { label: "Sally Stevens", url: "/2" },
    { label: "About", url: "/about" },
    { label: "Settings", url: "/settings" },
  ];
  links.forEach(({ label, url }) => {
    test(`Can navigate to ${label}`, async ({ page }) => {
      await page.getByRole("link", { name: label }).click();

      await expect(page.getByRole("heading", { name: label })).toBeVisible();
      await expect(page).toHaveURL(url);
    });
  });

  test("Can redirect to home", async ({ page }) => {
    await page.getByRole("link", { name: "About" }).click();

    await page.getByRole("link", { name: "Redirect to Home" }).click();
    await expect(page).toHaveURL("/");
  });
});
