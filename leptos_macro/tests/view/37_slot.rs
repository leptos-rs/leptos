// A `#[slot]` component passed to a parent component via `<HelloSlot slot>`.
// The slot has optional children. This should compile without errors.

use leptos::prelude::*;

#[slot]
struct HelloSlot<F: Fn() -> bool> {
    // Same prop syntax as components.
    concrete_i32: i32,

    generic_fun: F,

    #[prop(optional)]
    children: Option<Children>,
}

#[component]
fn HelloComponent<F: Fn() -> bool>(hello_slot: HelloSlot<F>) -> impl IntoView {
    let _ = hello_slot.concrete_i32;
    let _ = (hello_slot.generic_fun)();
    hello_slot.children.map(|children| children())
}

#[component]
fn App() -> impl IntoView {
    view! {
        <HelloComponent>
            <HelloSlot slot concrete_i32=42 generic_fun=|| true>
                "Hello, World!"
            </HelloSlot>
        </HelloComponent>
    }
}

fn main() {}
