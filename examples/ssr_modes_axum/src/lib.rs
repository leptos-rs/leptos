pub mod app;

#[cfg(feature = "ssr")]
pub mod fallback;

#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    use app::*;

    // initializes logging using the `log` crate
    //_ = console_log::init_with_level(log::Level::Debug);
    console_error_panic_hook::set_once();

    leptos::hydrate_body(App);
}
