pub mod app;
pub mod error_template;
pub mod errors;
#[cfg(feature = "ssr")]
pub mod fallback;
#[cfg(feature = "ssr")]
pub mod middleware;

#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    use crate::app::TodoApp;

    _ = console_log::init_with_level(log::Level::Error);
    console_error_panic_hook::set_once();

    leptos::mount_to_body(TodoApp);
}
