# Leptos Hacker News Example with Axum

This example uses the basic Hacker News example as its basis, but shows how to run the server side as WASM running in a JS environment. In this example, Deno is used as the runtime.

**NOTE**: This example is slightly out of date pending an update to [`axum-js-fetch`](https://github.com/seanaye/axum-js-fetch/), which was waiting on a version of `gloo-net` that uses `http` 1.0. It still works with Leptos 0.5 and Axum 0.6, but not with the versions of Leptos (0.6 and later) that support Axum 1.0.

## Server Side Rendering with Deno

To run the Deno version, run

```bash
deno task build
deno task start
```
