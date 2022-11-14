# Leptos Counter Isomorphic Example

This example demonstrates how to use a function isomorphically, to run a server side function from the browser and receive a result.

## Server Side Rendering With Hydration
To run it as a server side app with hydration, first you should run 
```bash
wasm-pack build --target=web --no-default-features --features=hydrate
```
to generate the Webassembly to provide hydration features for the server.
Then run the server with `cargo run` to serve the server side rendered HTML and the WASM bundle for hydration. 
```bash
cargo run --no-default-features --features=ssr`
```
> Note that if your hydration code changes, you will have to rerun the wasm-pack command above
> This should be temporary, and vastly improve once cargo-leptos becomes ready for prime time!

If for some reason you want to run it as a fully client side app, that can be done with the instructions below.
