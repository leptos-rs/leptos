# Comment to post on https://github.com/leptos-rs/leptos/issues/4610

---

I've created a minimal reproduction for the "callback removed before attaching" panic in WebView2/Tauri.

**Repository:** [leptos-webview2-repro](https://github.com/YOUR_USERNAME/leptos-webview2-repro) *(push the `Z:\src\leptos-webview2-repro` folder to a GitHub repo and replace YOUR_USERNAME)*

**Quick start:**
```bash
cd leptos-webview2-repro
cargo tauri dev
```

Then click **"Toggle rapidly (30x @ 5ms)"** repeatedly. In WebView2, this intermittently triggers the panic.

**What it does:**
- Leptos CSR (no SSR/hydrate)
- Tauri 2 + Wry/WebView2
- `<Show>` boundary that toggles rapidly via staggered `gloo_timers::Timeout` callbacks
- Child content has `on:click` handlers
- The rapid mount/unmount causes the race in `tachys/src/html/event.rs` where `self.cb` is `None` during attach

**Fix:** Replace the `.expect()` with a resilient `match` that returns a no-op handler instead of panicking.

---
