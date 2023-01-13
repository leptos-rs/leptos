# Getting Started

> The code for this chapter can be found [here](https://github.com/leptos-rs/leptos/tree/main/docs/book/project/ch02_getting_started).

The easiest way to get started using Leptos is to use [Trunk](https://trunkrs.dev/), as many of our [examples](https://github.com/leptos-rs/leptos/tree/main/examples) do. (Trunk is a simple build tool that includes a dev server.)

If you don’t already have it installed, you can install Trunk by running

```bash
cargo install --lock trunk
```

Create a basic Rust binary project

```bash
cargo init leptos-todo
```

Add `leptos` as a dependency to your `Cargo.toml` with the `csr` featured enabled. (That stands for “client-side rendering.” We’ll talk more about Leptos’s support for server-side rendering and hydration later.)

```toml
leptos = "0.0"
```

You’ll want to set up a basic `index.html` with the following content:

```html
{{#include ../project/ch02_getting_started/index.html}}
```

Let’s start with a very simple `main.rs`

```rust
{{#include ../project/ch02_getting_started/src/main.rs}}
```

Now run `trunk serve --open`. Trunk should automatically compile your app and open it in your default browser. If you make edits to `main.rs`, Trunk will recompile your source code and live-reload the page.
