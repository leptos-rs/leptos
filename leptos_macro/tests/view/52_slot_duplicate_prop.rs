// Duplicate prop on a slot.
// The slot code path uses `emit_error!` (in `slot_helper.rs`)
// instead of `compile_error!` (in `component_builder.rs`),
// so this tests that path independently.

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
            <HelloSlot slot concrete_i32=1 concrete_i32=2>
                "Hello, World!"
            </HelloSlot>
        </HelloComponent>
    }
}

fn main() {}
