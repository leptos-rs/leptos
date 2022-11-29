# Leptos Todo App Sqlite 

This example creates a basic todo app with an Actix backend that uses Leptos' server functions to call sqlx from the client and seamlessly run it on the server

## Client Side Rendering
To run it as a Client Side App, you can issue  `trunk serve --open` in the root. This will build the entire
app into one CSR bundle

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