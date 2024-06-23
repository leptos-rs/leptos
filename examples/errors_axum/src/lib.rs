pub mod error_template;
pub mod errors;
pub mod landing;

#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    use crate::landing::App;
    console_error_panic_hook::set_once();
    leptos::mount::hydrate_body(App);
}
