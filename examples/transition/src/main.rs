use leptos::*;
use transition::transition_tabs;

#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

pub fn main() {
    mount_to_body(transition_tabs)
}
