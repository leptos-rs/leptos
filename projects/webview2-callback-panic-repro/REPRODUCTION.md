# Reproduction Instructions for leptos-rs/leptos#4610

## Summary

This minimal reproduction demonstrates the `callback removed before attaching` panic that occurs in Leptos CSR when running inside WebView2 (Tauri/Wry) with rapid mount/unmount of DOM nodes that have `on:click` handlers.

## Prerequisites

- Rust (latest stable)
- [Trunk](https://trunkrs.dev/) (`cargo install trunk`)
- Windows (for WebView2) or system WebView

## Steps to Reproduce

### 1. Clone / use this repro

```bash
cd leptos-webview2-repro
```

### 2. Run in Tauri dev mode (WebView2)

```bash
cargo tauri dev
```

This will:
- Start Trunk to build and serve the WASM frontend
- Launch the Tauri app with WebView2

### 3. Trigger the panic

1. Click **"Toggle rapidly (30x @ 5ms)"** — this fires 30 staggered timeouts that toggle the `<Show>` boundary every 5ms
2. The `<Show>` content includes a button with `on:click`
3. In WebView2, the rapid mount/unmount can cause a race where tachys tries to attach a callback that was already consumed
4. **Without the fix:** Intermittent panic `callback removed before attaching` in the web console
5. **With the fix:** No panic; at worst a diagnostic log

### Alternative: Build and run release

```bash
cd frontend && trunk build --release && cd ..
cargo tauri build
# Run the built binary from target/release/
```

## What This Demonstrates

- Leptos 0.8.x CSR (no SSR, no hydrate)
- Tauri 2.x with Wry/WebView2
- `<Show>` with `when` that toggles rapidly
- Child elements with `on:click` handlers
- The race: callback consumed before DOM attachment in `tachys/src/html/event.rs`

## Fix

Replace `self.cb.expect("callback removed before attaching").take()` with a `match` that returns a no-op `RemoveEventHandler` when `self.cb` is `None`, and optionally log a diagnostic.
