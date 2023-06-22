import { test, expect } from '@playwright/test';
import { CountersPage } from './counters-page';

test('should_see_the_title', async ({ page }) => {
  const counters = new CountersPage(page);
  await counters.goto();

  await expect(page).toHaveTitle("Counters (Stable)");
});

test('should see the initial_values', async ({ page }) => {
  const counters = new CountersPage(page);
  await counters.goto();

  await expect(counters.total).toHaveText("0");
  await expect(counters.counters).toHaveText("0");
});
