# SSR Modes Example

This example shows the different "rendering modes" that can be used while server-side rendering an application.

## Getting Started

See the [Examples README](../README.md) for setup and run instructions.

## Server-Side Rendering Modes

1. **Synchronous**: Serve an HTML shell that includes `fallback` for any `Suspense`. Load data on the client, replacing `fallback` once they're loaded.

   - _Pros_: App shell appears very quickly: great TTFB (time to first byte).
   - _Cons_: Resources load relatively slowly; you need to wait for JS + Wasm to load before even making a request.

2. **Out-of-order streaming**: Serve an HTML shell that includes `fallback` for any `Suspense`. Load data on the **server**, streaming it down to the client as it resolves, and streaming down HTML for `Suspense` nodes.

   - _Pros_: Combines the best of **synchronous** and **`async`**, with a very fast shell and resources that begin loading on the server.
   - _Cons_: Requires JS for suspended fragments to appear in correct order. Weaker meta tag support when it depends on data that's under suspense (has already streamed down `<head>`)

3. **In-order streaming**: Walk through the tree, returning HTML synchronously as in synchronous rendering and out-of-order streaming until you hit a `Suspense`. At that point, wait for all its data to load, then render it, then the rest of the tree.

   - _Pros_: Does not require JS for HTML to appear in correct order.
   - _Cons_: Loads the shell more slowly than out-of-order streaming or synchronous rendering because it needs to pause at every `Suspense`. Cannot begin hydration until the entire page has loaded, so earlier pieces
     of the page will not be interactive until the suspended chunks have loaded.

4. **`async`**: Load all resources on the server. Wait until all data are loaded, and render HTML in one sweep.
   - _Pros_: Better handling for meta tags (because you know async data even before you render the `<head>`). Faster complete load than **synchronous** because async resources begin loading on server.
   - _Cons_: Slower load time/TTFB: you need to wait for all async resources to load before displaying anything on the client.

## Quick Start

Run `cargo leptos watch` to run this example.
