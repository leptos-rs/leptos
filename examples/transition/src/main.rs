use leptos::*;
use log::Level;
use transition::transition_tabs;

#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

pub fn main() {
    console_log::init_with_level(Level::Debug);

    mount_to_body(transition_tabs)
}
