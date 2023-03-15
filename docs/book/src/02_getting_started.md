# Getting Started

There are two basic paths to getting started with Leptos:

1. Client-side rendering with [Trunk](https://trunkrs.dev/)
2. Full-stack rendering with [`cargo-leptos`](https://github.com/leptos-rs/cargo-leptos)

For the early examples, it will be easiest to begin with Trunk. We’ll introduce
`cargo-leptos` a little later in this series.

If you don’t already have it installed, you can install Trunk by running

```bash
cargo install trunk
```

Create a basic Rust binary project

```bash
cargo init leptos-tutorial
```

> We recommend using `nightly` Rust, as it enables [a few nice features](https://github.com/leptos-rs/leptos#nightly-note). To use `nightly` Rust with WebAssembly, you can run
>
> ```bash
> rustup toolchain install nightly
> rustup default nightly
> rustup target add wasm32-unknown-unknown
> ```

`cd` into your new `leptos-tutorial` project and add `leptos` as a dependency

```bash
cargo add leptos
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
    mount_to_body(|cx| view! { cx,  <p>"Hello, world!"</p> })
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
