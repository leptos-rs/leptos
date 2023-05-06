# Introducing `cargo-leptos`

So far, we’ve just been running code in the browser and using Trunk to coordinate the build process and run a local development process. If we’re going to add server-side rendering, we’ll need to run our application code on the server as well. This means we’ll need to build two separate binaries, one compiled to native code and running the server, the other compiled to WebAssembly (WASM) and running in the user’s browser. Additionally, the server needs to know how to serve this WASM version (and the JavaScript required to initialize it) to the browser.

This is not an insurmountable task but it adds some complication. For convenience and an easier developer experience, we built the [`cargo-leptos`](https://github.com/leptos-rs/cargo-leptos) build tool. `cargo-leptos` basically exists to coordinate the build process for your app, handling recompiling the server and client halves when you make changes, and adding some built-in support for things like Tailwind, SASS, and testing.

Getting started is pretty easy. Just run

```bash
cargo install cargo-leptos
```

And then to create a new project, you can run either

```bash
# for an Actix template
cargo leptos new --git leptos-rs/start
```

or

```bash
# for an Axum template
cargo leptos new --git leptos-rs/start-axum
```

Now `cd` into the directory you’ve created and run

```bash
cargo leptos watch
```

Once your app has compiled you can open up your browser to [`http://localhost:3000`](http://localhost:3000) to see it.

`cargo-leptos` has lots of additional features and built in tools. You can learn more [in its `README`](https://github.com/leptos-rs/cargo-leptos/blob/main/README.md).

But what exactly is happening when you open our browser to `localhost:3000`? Well, read on to find out.
