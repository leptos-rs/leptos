# Optimizing WASM Binary Size

One of the primary downsides of deploying a Rust/WebAssembly frontend app is that splitting a WASM file into smaller chunks to be dynamically loaded is significantly more difficult than splitting a JavaScript bundle. There have been experiments like [`wasm-split`](https://emscripten.org/docs/optimizing/Module-Splitting.html) in the Emscripten ecosystem but at present there’s no way to split and dynamically load a Rust/`wasm-bindgen` binary. This means that the whole WASM binary needs to be loaded before your app becomes interactive. Because the WASM format is designed for streaming compilation, WASM files are much faster to compile per kilobyte than JavaScript files. (For a deeper look, you can [read this great article from the Mozilla team](https://hacks.mozilla.org/2018/01/making-webassembly-even-faster-firefoxs-new-streaming-and-tiering-compiler/) on streaming WASM compilation.)

Still, it’s important to ship the smallest WASM binary to users that you can, as it will reduce their network usage and make your app interactive as quickly as possible.

So what are some practical steps?

## Things to Do

1. Make sure you’re looking at a release build. (Debug builds are much, much larger.)
2. Add a release profile for WASM that optimizes for size, not speed.

For a `cargo-leptos` project, for example, you can add this to your `Cargo.toml`:

```toml
[profile.wasm-release]
inherits = "release"
opt-level = 'z'
lto = true
codegen-units = 1

# ....

[package.metadata.leptos]
# ....
lib-profile-release = "wasm-release"
```

This will hyper-optimize the WASM for your release build for size, while keeping your server build optimized for speed. (For a pure client-rendered app without server considerations, just use the `[profile.wasm-release]` block as your `[profile.release]`.)

3. Always serve compressed WASM in production. WASM tends to compress very well, typically shrinking to less than 50% its uncompressed size, and it’s trivial to enable compression for static files being served from Actix or Axum.

4. If you’re using nightly Rust, you can rebuild the standard library with this same profile rather than the prebuilt standard library that’s distributed with the `wasm32-unknown-unknown` target.

To do this, create a file in your project at `.cargo/config.toml`

```toml
[unstable]
build-std = ["std", "panic_abort", "core", "alloc"]
build-std-features = ["panic_immediate_abort"]
```

Note that if you're using this with SSR too, the same Cargo profile will be applied. You'll need to explicitly specify your target:
```toml
[build]
target = "x86_64-unknown-linux-gnu" # or whatever
```

Also note that in some cases, the cfg feature `has_std` will not be set, which may cause build errors with some dependencies which check for `has_std`. You may fix any build errors due to this by adding:
```toml
[build]
rustflags = ["--cfg=has_std"]
```

And you'll need to add `panic = "abort"` to `[profile.release]` in `Cargo.toml`. Note that this applies the same `build-std` and panic settings to your server binary, which may not be desirable. Some further exploration is probably needed here.

5. One of the sources of binary size in WASM binaries can be `serde` serialization/deserialization code. Leptos uses `serde` by default to serialize and deserialize resources created with `create_resource`. You might try experimenting with the `miniserde` and `serde-lite` features, which allow you to use those crates for serialization and deserialization instead; each only implements a subset of `serde`’s functionality, but typically optimizes for size over speed.

## Things to Avoid

There are certain crates that tend to inflate binary sizes. For example, the `regex` crate with its default features adds about 500kb to a WASM binary (largely because it has to pull in Unicode table data!). In a size-conscious setting, you might consider avoiding regexes in general, or even dropping down and calling browser APIs to use the built-in regex engine instead. (This is what `leptos_router` does on the few occasions it needs a regular expression.)

In general, Rust’s commitment to runtime performance is sometimes at odds with a commitment to a small binary. For example, Rust monomorphizes generic functions, meaning it creates a distinct copy of the function for each generic type it’s called with. This is significantly faster than dynamic dispatch, but increases binary size. Leptos tries to balance runtime performance with binary size considerations pretty carefully; but you might find that writing code that uses many generics tends to increase binary size. For example, if you have a generic component with a lot of code in its body and call it with four different types, remember that the compiler could include four copies of that same code. Refactoring to use a concrete inner function or helper can often maintain performance and ergonomics while reducing binary size.

## A Final Thought

Remember that in a server-rendered app, JS bundle size/WASM binary size affects only _one_ thing: time to interactivity on the first load. This is very important to a good user experience: nobody wants to click a button three times and have it do nothing because the interactive code is still loading — but it's not the only important measure.

It’s especially worth remembering that streaming in a single WASM binary means all subsequent navigations are nearly instantaneous, depending only on any additional data loading. Precisely because your WASM binary is _not_ bundle split, navigating to a new route does not require loading additional JS/WASM, as it does in nearly every JavaScript framework. Is this copium? Maybe. Or maybe it’s just an honest trade-off between the two approaches!

Always take the opportunity to optimize the low-hanging fruit in your application. And always test your app under real circumstances with real user network speeds and devices before making any heroic efforts.
