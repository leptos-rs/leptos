pub mod app;
mod issue_4005;
mod issue_4088;
mod issue_4217;
mod pr_4015;
mod pr_4091;

#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    use app::*;
    console_error_panic_hook::set_once();
    leptos::mount::hydrate_body(App);
}
