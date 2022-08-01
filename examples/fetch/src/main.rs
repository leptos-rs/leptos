use fetch::fetch_example;
use leptos::*;

#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

pub fn main() {
    mount_to_body(fetch_example)
}
