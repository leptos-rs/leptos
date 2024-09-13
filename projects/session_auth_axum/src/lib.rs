pub mod auth;
pub mod error_template;
pub mod errors;
#[cfg(feature = "ssr")]
pub mod state;
pub mod todo;

#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    use crate::todo::*;
    _ = console_log::init_with_level(log::Level::Debug);
    console_error_panic_hook::set_once();

    leptos::mount::hydrate_body(TodoApp);
}
