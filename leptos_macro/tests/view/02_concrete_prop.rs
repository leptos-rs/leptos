// Correct usage of a component with a single concrete (non-generic) prop.
// This should compile without errors.

use leptos::prelude::*;

#[component]
fn ConcretePropsCorrect() -> impl IntoView {
    view! {
        <div>
            <Inner concrete_bool=true/>
        </div>
    }
}

#[component]
fn Inner(concrete_bool: bool) -> impl IntoView {
    let _ = concrete_bool;
    ()
}

fn main() {}
