use app::*;
use leptos::*;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen]
pub fn hydrate() {
    // initializes logging using the `log` crate
    // _ = console_log::init_with_level(tracing::Level::Debug);
    console_error_panic_hook::set_once();

    leptos::mount_to_body(App);
}
