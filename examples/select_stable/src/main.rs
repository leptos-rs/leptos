use leptos::*;
use select_stable::Selector;
use select_stable::Dynamic_selector;

fn main() {
    _ = console_log::init_with_level(log::Level::Debug);
    console_error_panic_hook::set_once();
    mount_to_body(|| view! { <Selector/> <Dynamic_selector/> })
}
