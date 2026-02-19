use leptos::prelude::*;

// One prop has the wrong type and another required prop is missing.
// The wrong-type prop produces E0277 + E0599 ({error}). Thanks to
// the presence builder, the missing-prop error is reported
// independently of {error} contamination. All three errors are
// visible simultaneously.

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
