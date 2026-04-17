use leptos::prelude::*;

// Multiple required props are missing.
// We expect one error per missing required prop, each on the component name.

#[component]
fn MultipleMissing() -> impl IntoView {
    view! {
        <div>
            <Inner/>
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
