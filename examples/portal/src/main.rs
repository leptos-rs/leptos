use leptos::*;
use portal::App;
use wasm_bindgen::JsCast;

fn main() {
    _ = console_log::init_with_level(log::Level::Debug);
    console_error_panic_hook::set_once();
    mount_to(
        leptos::document()
            .get_element_by_id("app")
            .unwrap()
            .unchecked_into(),
        || view! { <App/> },
    )
}
