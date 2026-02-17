// A `#[slot]` component passed to a parent component via `<HelloSlot slot>`.
// The slot has optional children. This should compile without errors.

use leptos::prelude::*;

#[slot]
struct HelloSlot {
    // Same prop syntax as components.
    #[prop(optional)]
    children: Option<Children>,
}

#[component]
fn HelloComponent(hello_slot: HelloSlot) -> impl IntoView {
    hello_slot.children.map(|children| children())
}

#[component]
fn App() -> impl IntoView {
    view! {
        <HelloComponent>
            <HelloSlot slot>
                "Hello, World!"
            </HelloSlot>
        </HelloComponent>
    }
}

fn main() {}
