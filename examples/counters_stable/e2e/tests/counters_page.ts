import { expect, Locator, Page } from "@playwright/test";

export class CountersPage {
  readonly page: Page;
  readonly addCounterButton: Locator;
  readonly addOneThousandCountersButton: Locator;
  readonly clearCountersButton: Locator;
  readonly decrementCountButton: Locator;
  readonly incrementCountButton: Locator;

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

    this.total = page.getByTestId("total");

    this.counters = page.getByTestId("counters");
  }

  async goto() {
    await this.page.goto("http://localhost:8080/");
  }

  async addCounter() {
    this.addCounterButton.click();
  }

  async addOneThousandCounters() {
    this.addOneThousandCountersButton.click();
  }

  async decrementCount() {
    this.decrementCountButton.click();
  }

  async incrementCount() {
    this.incrementCountButton.click();
  }

  async clearCounters() {
    this.clearCountersButton.click();
  }
}
