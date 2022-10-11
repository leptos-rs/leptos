use leptos::*;
use todomvc::*;
use wasm_bindgen::prelude::wasm_bindgen;

extern crate wee_alloc;
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[wasm_bindgen]
pub fn main() {
    console_log::init_with_level(log::Level::Debug);
    log::debug!("initialized logging");

    leptos::hydrate(body().unwrap(), |cx| {
        // initial state — identical to server
        let todos = Todos(vec![
            Todo::new(cx, 0, "Buy milk".to_string()),
            Todo::new(cx, 1, "???".to_string()),
            Todo::new(cx, 2, "Profit!".to_string()),
        ]);

        view! { cx,  <TodoMVC todos=todos/> }
    });
}
