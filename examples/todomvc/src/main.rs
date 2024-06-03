use leptos::prelude::*;
pub use todomvc::*;
fn main() {
    _ = console_log::init_with_level(log::Level::Debug);
    console_error_panic_hook::set_once();
    mount_to_body(TodoMVC)
}
