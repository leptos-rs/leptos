# `wasm_split_macros`

This crate provides macros that are used along with the `wasm_split_helpers` crate, which allows you to indicate that certain functions are appropriate split points for lazy-loaded code.

A build tool that supports this approach (like `cargo-leptos`) can then split a WebAssembly (WASM) binary into multiple chunks, which will be lazy-loaded when a split function is called.

This crate was adapted from an original prototype, which you can find [here](https://github.com/jbms/wasm-split-prototype), with an in-depth description of the approach [here](https://github.com/rustwasm/wasm-bindgen/issues/3939).

This functionality is provided in Leptos by the `#[lazy]` and `#[lazy_route]` macros.
