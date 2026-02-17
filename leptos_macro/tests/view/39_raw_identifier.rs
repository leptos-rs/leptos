// Correct usage of a component having a prop whose name had to be prefixed with `r#`.
// This should compile without errors.

use leptos::prelude::*;

#[component]
fn UsingRawIdentifier() -> impl IntoView {
    view! {
        <div>
            <Inner r#type=true/>
        </div>
    }
}

#[component]
fn Inner(r#type: bool) -> impl IntoView {
    let _ = r#type;
    ()
}

fn main() {}
