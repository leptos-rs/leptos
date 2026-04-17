use leptos::prelude::*;

// Correct usage of a generic component with concrete, generic, and children props.
// This should compile without errors.

#[component]
fn CorrectGenericUsage() -> impl IntoView {
    view! {
        <div>
            <Inner concrete_i32=42 generic_fun=|| true>
                "foo"
            </Inner>
        </div>
    }
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
