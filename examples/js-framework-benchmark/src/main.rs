use js_framework_benchmark_leptos::App;
use leptos::{
    leptos_dom::helpers::document, mount::mount_to, wasm_bindgen::JsCast,
};

pub fn main() {
    console_error_panic_hook::set_once();
    let root = document().query_selector("#main").unwrap().unwrap();
    let handle = mount_to(root.unchecked_into(), App);
    handle.forget();
}
