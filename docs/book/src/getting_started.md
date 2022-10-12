# Getting Started

> The code for this chapter can be found [here](https://github.com/gbj/leptos/tree/main/docs/book/project/ch01_getting_started).

The easiest way to get started using Leptos is to use [Trunk](https://trunkrs.dev/), as many of our [examples](https://github.com/gbj/leptos/tree/main/examples) do.

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
leptos = { version = "0.0", features = ["csr"] }
```

You’ll want to set up a basic `index.html` with the following content:

```html
<!DOCTYPE html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>Leptos • Todos</title>
    <link data-trunk rel="rust" data-wasm-opt="z" />
  </head>
  <body></body>
</html>
```

Let’s start with a very simple `main.rs`

```rust
use leptos::*;

fn main() {
    mount_to_body(|cx| view! { cx,  <p>"Hello, world!"</p> })
}
```

Now run `trunk serve --open`. Trunk should automatically compile your app and open it in your default browser.
