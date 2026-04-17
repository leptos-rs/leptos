use leptos::prelude::*;

// This test fails to provide the required children.
// We expect an error to be reported on the component name `Inner`.

#[component]
fn ChildrenMissing() -> impl IntoView {
    view! {
        <div>
            <Inner concrete_i32=42 generic_fun=|| true>
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
