// Missing required prop on a slot.
// Error expected on the slot name, with "on slot" wording.

use leptos::prelude::*;

#[slot]
struct HelloSlot {
    concrete_i32: i32,

    #[prop(optional)]
    children: Option<Children>,
}

#[component]
fn HelloComponent(hello_slot: HelloSlot) -> impl IntoView {
    let _ = hello_slot.concrete_i32;
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
