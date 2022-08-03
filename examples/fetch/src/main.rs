use fetch::fetch_example;
use leptos::*;

#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

pub fn main() {
    console_log::init_with_level(log::Level::Debug);

    mount_to_body(fetch_example)
}
