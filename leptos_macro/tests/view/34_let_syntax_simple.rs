// Using `let:value` syntax to bind a component's output to a variable
// accessible in its children. This should compile without errors.

use leptos::prelude::*;

#[component]
pub fn MyComponent() -> impl IntoView {
    view! {
        <Inner concrete_i32=42 let:value>
            "concrete_i32 passed was: " { value }
        </Inner>
    }
}

#[component]
fn Inner<F, IV>(concrete_i32: i32, children: F) -> impl IntoView
where
    F: FnOnce(i32) -> IV + 'static,
    IV: IntoView,
{
    children(concrete_i32)
}

fn main() {}
