use leptos::prelude::*;
use spread::SpreadingExample;

pub fn main() {
    _ = console_log::init_with_level(log::Level::Debug);
    console_error_panic_hook::set_once();
    mount::mount_to_body(SpreadingExample)
}
