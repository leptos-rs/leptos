pub mod app;
pub mod collect;
pub mod islands;

#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    use leptos::prelude::*;

    console_error_panic_hook::set_once();
    leptos::mount::hydrate_islands();
    // Tells htmx-islands.js the wasm is ready: from now on it wakes the islands htmx swaps in.
    let _ = leptos::web_sys::js_sys::Reflect::set(
        &window(),
        &"__islandsReady".into(),
        &true.into(),
    );
}
