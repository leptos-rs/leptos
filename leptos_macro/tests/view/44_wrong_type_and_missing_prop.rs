use leptos::prelude::*;

// One prop has the wrong type and another required prop is missing.
// The wrong-type prop produces E0277 + E0599 ({error}), which
// suppresses the missing-prop error. This is expected behavior:
// fix the type error first, then the missing-prop error appears.

#[component]
fn WrongTypeAndMissing() -> impl IntoView {
    view! {
        <div>
            <Inner generic_fun=true>
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
