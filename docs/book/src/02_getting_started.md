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

Create a basic Rust project

```bash
cargo init leptos-tutorial
```

`cd` into your new `leptos-tutorial` project and add `leptos` as a dependency

```bash
cargo add leptos --features=csr,nightly
```

> **Note**: This version of the book reflects the Leptos 0.5 release. The CodeSandbox examples have not yet been updated from 0.4 and earlier versions.

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
