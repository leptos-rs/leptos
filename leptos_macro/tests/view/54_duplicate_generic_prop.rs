// Duplicate generic prop.
// Ensures duplicate detection fires before pre-checks,
// so only the duplicate error is shown.

use leptos::prelude::*;

#[component]
fn DuplicateGenericProp() -> impl IntoView {
    view! {
        <div>
            <Inner generic_fun={|| true} generic_fun={|| false}/>
        </div>
    }
}

#[component]
fn Inner<F>(generic_fun: F) -> impl IntoView
where
    F: Fn() -> bool,
{
    let _ = generic_fun();
    ()
}

fn main() {}
