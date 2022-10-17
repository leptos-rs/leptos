use leptos::*;
pub use todomvc::*;
fn main() {
    _ = console_log::init_with_level(log::Level::Debug);
    mount_to_body(|cx| view! { cx,  <TodoMVC todos=Todos::new(cx)/> })
}
