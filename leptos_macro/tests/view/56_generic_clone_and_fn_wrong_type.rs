// Generic prop bounded by `Clone + Fn() -> bool`.
// Tests whether the Fn hint appears when Fn is not the first bound.
// The `starts_with("Fn")` check in `classify_prop` operates on
// the token-stringified bounds, so if `Clone` comes first,
// the hint might not trigger.

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
