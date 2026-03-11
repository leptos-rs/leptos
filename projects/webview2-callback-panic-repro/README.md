# Minimal Reproduction: "callback removed before attaching" in WebView2/Tauri

Reproduction for [leptos-rs/leptos#4610](https://github.com/leptos-rs/leptos/issues/4610).  
Located at `projects/webview2-callback-panic-repro/` in the leptos repo.

## Prerequisites

- Rust (latest stable)
- [Trunk](https://trunkrs.dev/) (`cargo install trunk`)
- [Tauri CLI](https://tauri.app/) (`cargo install tauri-cli --version "^2"`)

## How to Reproduce

### Build (if `~/.cargo/config` has `-fuse-ld=lld`, override for wasm)
```powershell
$env:RUSTFLAGS=''
cd frontend
cargo build --release --target wasm32-unknown-unknown --config 'target.wasm32-unknown-unknown.rustflags=[]'
trunk build --release
cd ..
```

### Run
1. **Start dev server** (in one terminal):
   ```bash
   cd frontend && trunk serve
   ```

2. **Run Tauri** (in another terminal):
   ```bash
   cargo tauri dev
   ```
   (Config uses `beforeDevCommand: ""` and `devUrl: http://localhost:1420`)

3. **Trigger the panic:**
   - App auto-triggers 10 cycles of rapid toggles on load
   - Or click **"Toggle rapidly (30x @ 5ms)"** repeatedly
   - The `<Show>` boundary mounts/unmounts DOM nodes with `on:click` handlers
   - In WebView2, this race can cause: `callback removed before attaching`

4. **Expected (without fix):** Intermittent panic in WebView DevTools (Ctrl+Shift+I → Console)
5. **Expected (with fix):** No panic; at worst a diagnostic log

## What This Reproduces

- Leptos CSR (no SSR, no hydrate)
- Tauri 2.x with Wry/WebView2
- Rapid `<Show>` toggling with `on:click` handlers on child elements

## Leptos Version

Uses **unpatched** leptos from crates.io to demonstrate the bug. To test the fix, patch tachys as in the PR.
