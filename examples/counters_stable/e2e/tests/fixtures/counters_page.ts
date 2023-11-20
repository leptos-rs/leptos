import { expect, Locator, Page } from "@playwright/test";

export class CountersPage {
  readonly page: Page;
  readonly addCounterButton: Locator;
  readonly addOneThousandCountersButton: Locator;
  readonly clearCountersButton: Locator;

  readonly incrementCountButton: Locator;
  readonly counterInput: Locator;
  readonly decrementCountButton: Locator;
  readonly removeCountButton: Locator;

  readonly total: Locator;
  readonly counters: Locator;

  constructor(page: Page) {
    this.page = page;

    this.addCounterButton = page.locator("button", { hasText: "Add Counter" });

    this.addOneThousandCountersButton = page.locator("button", {
      hasText: "Add 1000 Counters",
    });

    this.clearCountersButton = page.locator("button", {
      hasText: "Clear Counters",
    });

    this.decrementCountButton = page.locator("button", {
      hasText: "-1",
    });

    this.incrementCountButton = page.locator("button", {
      hasText: "+1",
    });

    this.removeCountButton = page.locator("button", {
      hasText: "x",
    });

    this.total = page.getByTestId("total");

    this.counters = page.getByTestId("counters");

    this.counterInput = page.getByRole("textbox");
  }

  async goto() {
    await this.page.goto("/");
  }

  async addCounter() {
    await Promise.all([
      this.addCounterButton.waitFor(),
      this.addCounterButton.click(),
    ]);
  }

  async addOneThousandCounters() {
    this.addOneThousandCountersButton.click();
  }

  async decrementCount(index: number = 0) {
    await Promise.all([
      this.decrementCountButton.nth(index).waitFor(),
      this.decrementCountButton.nth(index).click(),
    ]);
  }

  async incrementCount(index: number = 0) {
    await Promise.all([
      this.incrementCountButton.nth(index).waitFor(),
      this.incrementCountButton.nth(index).click(),
    ]);
  }

  async clearCounters() {
    await Promise.all([
      this.clearCountersButton.waitFor(),
      this.clearCountersButton.click(),
    ]);
  }

  async enterCount(count: string, index: number = 0) {
    await Promise.all([
      this.counterInput.nth(index).waitFor(),
      this.counterInput.nth(index).fill(count),
    ]);
  }

  async removeCounter(index: number = 0) {
    await Promise.all([
      this.removeCountButton.nth(index).waitFor(),
      this.removeCountButton.nth(index).click(),
    ]);
  }
}
