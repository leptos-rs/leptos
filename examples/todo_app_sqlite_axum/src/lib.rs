pub mod error_template;
pub mod errors;
#[cfg(feature = "ssr")]
pub mod fallback;
pub mod todo;

#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    use crate::todo::TodoApp;

    _ = console_log::init_with_level(log::Level::Error);
    console_error_panic_hook::set_once();

    leptos::hydrate_body(TodoApp);
}
