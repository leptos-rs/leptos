import { expect, Locator, Page } from "@playwright/test";

export class CountersPage {
  readonly page: Page;
  readonly addCounterButton: Locator;
  readonly addOneThousandCountersButton: Locator;
  readonly clearCountersButton: Locator;
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
    this.total = page.locator("#total");
    this.counters = page.locator("#counters");
  }

  async goto() {
    await this.page.goto("http://localhost:8080/");
  }

  async addCounter() {
    this.addCounterButton.first().click();
  }

  async addOneThousandCounters() {
    this.addOneThousandCountersButton.first().click();
  }

  async clearCounters() {
    this.clearCountersButton.first().click();
  }
}
