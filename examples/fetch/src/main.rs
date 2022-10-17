use fetch::fetch_example;
use leptos::*;

pub fn main() {
    _ = console_log::init_with_level(log::Level::Debug);

    mount_to_body(fetch_example)
}
