import { expect, Locator, Page } from "@playwright/test";

export class CountersPage {
  private page: Page;
  private addCounterButton: Locator;
  private addOneThousandCountersButton: Locator;
  private clearCountersButton: Locator;
  private decrementCountButton: Locator;
  private incrementCountButton: Locator;
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

  async decrementCount() {
    this.decrementCountButton.first().click();
  }

  async incrementCount() {
    this.incrementCountButton.first().click();
  }
}
