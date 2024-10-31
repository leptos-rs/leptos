use leptos::prelude::*;
use portal::App;
use wasm_bindgen::JsCast;

fn main() {
    _ = console_log::init_with_level(log::Level::Debug);
    console_error_panic_hook::set_once();
    let handle = mount_to(
        document()
            .get_element_by_id("app")
            .unwrap()
            .unchecked_into(),
        App,
    );
    handle.forget();
}
