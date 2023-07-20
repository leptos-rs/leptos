use counter_without_macros::counter;
use leptos::*;

/// Show the counter
pub fn main() {
    _ = console_log::init_with_level(log::Level::Debug);
    console_error_panic_hook::set_once();
    mount_to_body(|| counter(0, 1))
}
