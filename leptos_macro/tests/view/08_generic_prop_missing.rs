use leptos::prelude::*;

// Missing required generic prop `generic_fun`.
// We expect the error to be on the component name `Inner`.

#[component]
fn GenericPropMissing() -> impl IntoView {
    view! {
        <div>
            <Inner concrete_i32=42>
                "foo"
            </Inner>
        </div>
    }
}

#[component]
fn Inner<F>(concrete_i32: i32, generic_fun: F, children: ChildrenFn) -> impl IntoView
where
    F: Fn() -> bool,
{
    let _ = concrete_i32;
    let _ = generic_fun();
    children()
}

fn main() {}
