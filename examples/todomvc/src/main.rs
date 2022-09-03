use leptos::*;
use todomvc::{TodoMVC, TodoMVCProps};

#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

fn main() {
    _ = console_log::init_with_level(log::Level::Debug);
    mount_to_body(|cx| view! { <TodoMVC/> })
}
