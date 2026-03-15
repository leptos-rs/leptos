// Component where `children` is the only required prop, and it's missing.
// Verifies the note says "add child elements between the opening and
// closing tags" in isolation (without other required props).

use leptos::prelude::*;

#[component]
fn ChildrenOnlyMissing() -> impl IntoView {
    view! {
        <div>
            <Inner></Inner>
        </div>
    }
}

#[component]
fn Inner(children: Children) -> impl IntoView {
    children()
}

fn main() {}
