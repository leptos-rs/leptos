# islands_router e2e tests

Playwright tests for the islands router's client-side navigation, including a
regression test for `replaceBranch`'s branch counting when a route view starts
with marker comments instead of an element.

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
