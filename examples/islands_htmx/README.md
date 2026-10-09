# Islands and htmx

Leptos islands in an Axum app that swaps pages and fragments with htmx.

## The problem

Leptos wakes the islands on a page once, when the page loads. htmx swaps new
HTML in without a page load, so two things go wrong:

1. An island htmx **swaps in** stays dead. Leptos never sees it, so its
   buttons do nothing.
2. An island htmx **swaps out** is never freed. The `#[island]` macro keeps
   every island it wakes in memory for good (`std::mem::forget`, with a
   `TODO better cleanup` next to it). Each swap leaves the old island's
   signals, effects and listeners behind until the next page load. Leptos's
   own islands router swaps nodes the same way.

This example fixes both from the outside, without changing Leptos.

## How this was made

Built with Claude (Anthropic's AI assistant), from code first written for
another app. Reviewed and tested by hand.

## Where things are

| File | What it does |
|---|---|
| `public/htmx-islands.js` | The glue. Wakes every island htmx swaps in, and keeps htmx from dropping the HTML comments Leptos needs to wake them |
| `src/collect.rs` | The collector. Frees an island a few seconds after it has left the page |
| `src/islands.rs` | Two islands: `Counter` with state, `KeyLog` with a window listener it removes when freed |
| `src/app.rs` | The pages, and `PageLink`, the htmx link that swaps `<main>` |
| `src/main.rs` | The Axum server, and `/fragment`, a piece of HTML that holds an island |
| `src/collect.rs`, at the end | The collector's tests, in headless Chrome |
| `e2e/tests/htmx.spec.ts` | The end-to-end tests, with Playwright |

## Run it

```sh
# From the repo root
cd examples/islands_htmx && cargo leptos watch   # then http://127.0.0.1:3000
```

1. On Home, press a key: the island shows it.
2. Go to About: `<main>` is swapped, the page does not reload.
3. Click "Load a counter", then click the counter: it counts.
4. In a debug build the console shows `islands: 0 live, 1 freed` a few
   seconds after Home's island has left the page.

## Testing

```sh
# From the repo root

# The collector's tests, in headless Chrome
cd examples/islands_htmx && wasm-pack test --headless --chrome -- --lib --features hydrate

# The end-to-end tests: builds, serves and runs Playwright
# Stop `cargo leptos watch` first: both use port 3000
cd examples/islands_htmx && cargo leptos end-to-end
```

The collector's tests check that an island off the page is freed after two
sweeps, kept while on the page, kept when it comes back, and still freed in a
debug build, which erases component types.

The end-to-end tests check that a link swaps the page without a reload, that
the fragment's island counts, that a swapped-out island is freed with its
window listener, and that Back reloads the page.
