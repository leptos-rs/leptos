import { expect, Locator, Page } from "@playwright/test";

export class HomePage {
  readonly page: Page;
  readonly pageTitle: Locator;
  readonly numberInput: Locator;
  readonly successMessage: Locator;
  readonly errorMessage: Locator;

  readonly errorList: Locator;

  constructor(page: Page) {
    this.page = page;

    this.pageTitle = page.locator("h1");
    this.numberInput = page.getByLabel(
      "Type an integer (or something that's not an integer!)"
    );
    this.successMessage = page.locator("label p");
    this.errorMessage = page.locator("div p");
    this.errorList = page.getByRole("list");
  }

  async goto() {
    await this.page.goto("/");
  }

  async enterNumber(count: string, index: number = 0) {
    await Promise.all([
      this.numberInput.waitFor(),
      this.numberInput.fill(count),
    ]);
  }

  async clickUpArrow() {
    await Promise.all([
      this.numberInput.waitFor(),
      this.numberInput.press("ArrowUp"),
    ]);
  }

  async clickDownArrow() {
    await Promise.all([
      this.numberInput.waitFor(),
      this.numberInput.press("ArrowDown"),
    ]);
  }

  async clearInput() {
    await Promise.all([
      this.numberInput.waitFor(),
      this.clickUpArrow(),
      this.numberInput.press("Backspace"),
    ]);
  }
}
