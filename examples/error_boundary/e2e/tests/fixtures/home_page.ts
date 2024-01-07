import { expect, Locator, Page } from "@playwright/test";

export class HomePage {
  readonly page: Page;
  readonly pageTitle: Locator;
  readonly valueInput: Locator;
  readonly successMessage: Locator;
  readonly errorMessage: Locator;

  readonly errorList: Locator;

  constructor(page: Page) {
    this.page = page;

    this.pageTitle = page.locator("h1");
    this.valueInput = page.getByLabel(
      "Type a number (or something that's not a number!)"
    );
    this.successMessage = page.locator("label p");
    this.errorMessage = page.locator("div p");
    this.errorList = page.getByRole("list");
  }

  async goto() {
    await this.page.goto("/");
  }

  async enterValue(value: string) {
    await Promise.all([
      this.valueInput.waitFor(),
      this.valueInput.fill(value),
    ]);
  }

  async clearInput() {
    await Promise.all([
      this.valueInput.waitFor(),
      this.valueInput.press("Backspace"),
    ]);
  }
}
