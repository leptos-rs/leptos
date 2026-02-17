// Wrong type for a generic prop on a `#[slot]` component.
// Passes `generic_fun=true` (a `bool`) where `Fn() -> bool` is required.
// Error expected on `generic_fun=true`.

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
    hello_slot.children.map(|children| children())
}

#[component]
fn App() -> impl IntoView {
    view! {
        <HelloComponent>
            <HelloSlot slot concrete_i32=42 generic_fun=true>
                "Hello, World!"
            </HelloSlot>
        </HelloComponent>
    }
}

fn main() {}
