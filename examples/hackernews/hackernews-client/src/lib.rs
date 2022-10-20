use hackernews_app::*;
use leptos::*;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen]
pub fn main() {
    _ = console_log::init_with_level(log::Level::Debug);
    console_error_panic_hook::set_once();
    leptos::hydrate(body().unwrap(), move |cx| {
        view! { cx, <App/> }
    });
}
