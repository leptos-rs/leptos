use leptos::*;
use router::router_example;

pub fn main() {
    _ = console_log::init_with_level(log::Level::Debug);
    mount_to_body(router_example)
}
