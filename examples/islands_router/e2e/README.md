# islands_router e2e tests

Playwright tests for the islands router's client-side navigation, including
regression coverage for navigating away from the Home route, whose `<Title/>`
metadata component renders no element in the route body.

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
