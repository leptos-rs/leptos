# Getting Started

There are two basic paths to getting started with Leptos:

1. **Client-side rendering (CSR) with [Trunk](https://trunkrs.dev/)** - a great option if you just want to make a snappy website with Leptos, or work with a pre-existing server or API.
In CSR mode, Trunk compiles your Leptos app to WebAssembly (WASM) and runs it in the browser like a typical Javascript single-page app (SPA). The advantages of Leptos CSR include faster build times and a quicker iterative development cycle, as well as a simpler mental model and more options for deploying your app. CSR apps do come with some disadvantages: initial load times for your end users are slower compared to a server-side rendering approach, and the usual SEO challenges that come along with using a JS single-page app model apply to Leptos CSR apps as well. Also note that, under the hood, an auto-generated snippet of JS is used to load the Leptos WASM bundle, so JS *must* be enabled on the client device for your CSR app to display properly. As with all software engineering, there are trade-offs here you'll need to consider.

2. **Full-stack, server-side rendering (SSR) with [`cargo-leptos`](https://github.com/leptos-rs/cargo-leptos)** - SSR is a great option for building CRUD-style websites and custom web apps if you want Rust powering both your frontend and backend.
With the Leptos SSR option, your app is rendered to HTML on the server and sent down to the browser; then, WebAssembly is used to instrument the HTML so your app becomes interactive - this process is called 'hydration'. On the server side, Leptos SSR apps integrate closely with your choice of either [Actix-web](https://docs.rs/leptos_actix/latest/leptos_actix/index.html) or [Axum](https://docs.rs/leptos_axum/latest/leptos_axum/index.html) server libraries, so you can leverage those communities' crates to help build out your Leptos server.
The advantages of taking the SSR route with Leptos include helping you get the best initial load times and optimal SEO scores for your web app. SSR apps can also dramatically simplify working across the server/client boundary via a Leptos feature called "server functions", which lets you transparently call functions on the server from your client code (more on this feature later). Full-stack SSR isn't all rainbows and butterflies, though - disadvantages include a slower developer iteration loop (because you need to recompile both the server and client when making Rust code changes), as well as some added complexity that comes along with hydration.

By the end of the book, you should have a good idea of which trade-offs to make and which route to take - CSR or SSR - depending on your project's requirements.


In Part 1 of this book, we'll start with client-side rendering Leptos sites and building reactive UI's using `Trunk` to serve our JS and WASM bundle to the browser.

We’ll introduce `cargo-leptos` in Part 2 of this book, which is all about working with the full power of Leptos in its full-stack, SSR mode.

```admonish note
If you're coming from the Javascript world and terms like client-side rendering (CSR) and server-side rendering (SSR) are unfamiliar to you, the easiest way to understand the difference is by analogy:

Leptos' CSR mode is similar to working with React (or a 'signals'-based framework like SolidJS), and focuses on producing a client-side UI which you can use with any tech stack on the server.

Using Leptos' SSR mode is similar to working with a full-stack framework like Next.js in the React world (or Solid's "SolidStart" framework) - SSR helps you build sites and apps that are rendered on the server then sent down to the client. SSR can help to improve your site's loading performance and accessibility as well as make it easier for one person to work on *both* client- and server-side without needing to context-switch between different languages for frontend and backend.

The Leptos framework can be used either in CSR mode to just make a UI (like React), or you can use Leptos in full-stack SSR mode (like Next.js) so that you can build both your UI and your server with one language: Rust.

```

## Hello World! Getting Set up for Leptos CSR Development

First up, make sure Rust is installed and up-to-date ([see here if you need instructions](https://www.rust-lang.org/tools/install)).

If you don’t have it installed already, you can install the "Trunk" tool for running Leptos CSR sites by running the following on the command-line:

```bash
cargo install trunk
```

And then create a basic Rust project

```bash
cargo init leptos-tutorial
```

`cd` into your new `leptos-tutorial` project and add `leptos` as a dependency

```bash
cargo add leptos --features=csr,nightly
```

Or you can leave off `nightly` if you're using stable Rust

```bash
cargo add leptos --features=csr
```

> Using `nightly` Rust, and the `nightly` feature in Leptos enables the function-call syntax for signal getters and setters that is used in most of this book.
>
> To use nightly Rust, you can either opt into nightly for all your Rust projects by running
>
> ```bash
> rustup toolchain install nightly
> rustup default nightly
> ```
>
> or only for this project
>
> ```bash
> rustup toolchain install nightly
> cd <into your project>
> rustup override set nightly
> ```
>
> [See here for more details.](https://doc.rust-lang.org/book/appendix-07-nightly-rust.html)
>
> If you’d rather use stable Rust with Leptos, you can do that too. In the guide and examples, you’ll just use the [`ReadSignal::get()`](https://docs.rs/leptos/latest/leptos/struct.ReadSignal.html#impl-SignalGet%3CT%3E-for-ReadSignal%3CT%3E) and [`WriteSignal::set()`](https://docs.rs/leptos/latest/leptos/struct.WriteSignal.html#impl-SignalGet%3CT%3E-for-ReadSignal%3CT%3E) methods instead of calling signal getters and setters as functions.

Make sure you've added the `wasm32-unknown-unknown` target so that Rust can compile your code to WebAssembly to run in the browser.

```bash
rustup target add wasm32-unknown-unknown
```

Create a simple `index.html` in the root of the `leptos-tutorial` directory

```html
<!DOCTYPE html>
<html>
  <head></head>
  <body></body>
</html>
```

And add a simple “Hello, world!” to your `main.rs`

```rust
use leptos::*;

fn main() {
    mount_to_body(|| view! { <p>"Hello, world!"</p> })
}
```

Your directory structure should now look something like this

```
leptos_tutorial
├── src
│   └── main.rs
├── Cargo.toml
├── index.html
```

Now run `trunk serve --open` from the root of the `leptos-tutorial` directory.
Trunk should automatically compile your app and open it in your default browser.
If you make edits to `main.rs`, Trunk will recompile your source code and
live-reload the page.


Welcome to the world of UI development with Rust and WebAssembly (WASM), powered by Leptos and Trunk!

---

Now before we get started building your first real UI's with Leptos, there are a couple of things you might want to know to help make your experience with Leptos just a little bit easier.