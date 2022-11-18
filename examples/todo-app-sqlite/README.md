# Leptos Counter Isomorphic Example

This example demonstrates how to use a server functions and multi-actions to build a simple todo app.

## Server Side Rendering With Hydration

To run it as a server side app with hydration, first you should run

```bash
wasm-pack build --target=web --no-default-features --features=hydrate
```

to generate the Webassembly to provide hydration features for the server.
Then run the server with `cargo run` to serve the server side rendered HTML and the WASM bundle for hydration.

```bash
cargo run --no-default-features --features=ssr
```

> Note that if your hydration code changes, you will have to rerun the wasm-pack command above
> This should be temporary, and vastly improve once cargo-leptos becomes ready for prime time!
