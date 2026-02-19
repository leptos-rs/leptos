// Generic prop with non-Fn bound (`Clone + Display`).
// Verifies the "a closure or function reference" hint is NOT shown
// for non-Fn bounds.

use leptos::prelude::*;

#[component]
fn NonFnBoundWrongType() -> impl IntoView {
    view! {
        <div>
            <Inner value=vec![1, 2, 3]/>
        </div>
    }
}

#[component]
fn Inner<T>(value: T) -> impl IntoView
where
    T: Clone + std::fmt::Display,
{
    let _ = value.to_string();
    ()
}

fn main() {}
