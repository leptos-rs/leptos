pub mod error_template;
pub mod errors;
pub mod todo;

#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    use crate::todo::TodoApp;
    console_error_panic_hook::set_once();
    leptos::mount::hydrate_body(TodoApp);
}
