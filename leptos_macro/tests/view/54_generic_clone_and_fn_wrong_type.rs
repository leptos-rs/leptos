// Generic prop bounded by `Clone + Fn() -> bool`.
// Tests that the Fn hint appears even when Fn is not the first bound.

use leptos::prelude::*;

#[component]
fn CloneAndFnWrongType() -> impl IntoView {
    view! {
        <div>
            <Inner fun=true/>
        </div>
    }
}

#[component]
fn Inner<F>(fun: F) -> impl IntoView
where
    F: Clone + Fn() -> bool,
{
    let _ = fun();
    ()
}

fn main() {}
