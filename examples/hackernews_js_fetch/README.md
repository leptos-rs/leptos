# Leptos Hacker News Example with Axum

This example uses the basic Hacker News example as its basis, but shows how to run the server side as WASM running in a JS environment. In this example, Deno is used as the runtime. 

## Client Side Rendering
To run it as a Client Side App, you can issue  `trunk serve --open` in the root. This will build the entire
app into one CSR bundle. Make sure you have trunk installed with `cargo install trunk`.

## Server Side Rendering with Deno
To run the Deno version, run
```bash
deno task build
deno task start 
```
