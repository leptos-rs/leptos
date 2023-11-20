use js_framework_benchmark_leptos::App;
use leptos::{wasm_bindgen::JsCast, *};

pub fn main() {
    _ = console_log::init_with_level(log::Level::Debug);
    console_error_panic_hook::set_once();

    let root = document().query_selector("#main").unwrap().unwrap();
    mount_to(root.unchecked_into(), || view! { <App/> });
}
