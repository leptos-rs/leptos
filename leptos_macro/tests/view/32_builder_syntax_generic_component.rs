// Invoking a generic component via `.builder()` syntax, passing concrete,
// generic, and children props. This should compile without errors.

use leptos::{html::p, prelude::*};

#[component]
pub fn MyComponent() -> impl IntoView {
    Inner(
        InnerProps::builder()
            .concrete_i32(42)
            .generic_fun(|| true)
            .children(ToChildren::to_children(|| p().child("Child")))
            .build(),
    )
}

#[component]
fn Inner<F>(
    concrete_i32: i32,
    generic_fun: F,
    children: ChildrenFn,
) -> impl IntoView
where
    F: Fn() -> bool,
{
    let _ = concrete_i32;
    let _ = generic_fun();
    children()
}

fn main() {}
