# islands_htmx e2e tests

Playwright tests for the islands_htmx example: a link swaps the page without
a load, a swapped-in island wakes, a swapped-out island is freed, and Back
loads the page.

## Running

Serve the example (from the example root):

```sh
cargo leptos serve
```

Then, in this directory:

```sh
npm install
npx playwright install chromium
npx playwright test
```
