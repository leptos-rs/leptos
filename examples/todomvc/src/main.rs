use leptos::*;
use todomvc::{TodoMVC, TodoMVCProps};

#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

fn main() {
    mount_to_body(|cx| view! { <TodoMVC/> })
}
