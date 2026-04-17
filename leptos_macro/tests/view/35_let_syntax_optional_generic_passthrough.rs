// Using `let:value` with a generic component whose generic prop is
// `Option<T>`. Passes `Some(42)` and binds the result in children.
// This should compile without errors.

use leptos::prelude::*;

#[component]
pub fn MyComponent() -> impl IntoView {
    view! {
        <Inner optional_generic=Some(42) let:value>
            "optional generic value passed was: " { value }
        </Inner>
    }
}

#[component]
fn Inner<T, F, IV>(optional_generic: Option<T>, children: F) -> impl IntoView
where
    F: FnOnce(T) -> IV + 'static,
    IV: IntoView,
{
    match optional_generic {
        Some(value) => children(value).into_any(),
        None => ().into_any(),
    }
}

fn main() {}
