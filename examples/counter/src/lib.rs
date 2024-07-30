use leptos::prelude::*;

/// A simple counter component.
///
/// You can use doc comments like this to document your component.
#[component]
pub fn SimpleCounter(
    /// The starting value for the counter
    initial_value: i32,
    /// The change that should be applied each time the button is clicked.
    step: i32,
) -> impl IntoView {
    /*let (value, set_value) = signal(initial_value);

    view! {
        <div>
            <button on:click=move |_| set_value.set(0)>"Clear"</button>
            <button on:click=move |_| *set_value.write() -= step>"-1"</button>
            <span>"Value: " {value} "!"</span>
            <button on:click=move |_| set_value.update(|value| *value += step)>"+1"</button>
        </div>
    }*/
    App()
}

use gloo_timers::future::TimeoutFuture;
use leptos::{html::Input, prelude::*};

#[component]
fn Widget() -> impl IntoView {
    let input_ref = NodeRef::<Input>::new();

    Effect::new(move |_| {
        let Some(_) = input_ref.get() else {
            log!("no ref");
            return;
        };
        log!("ref");
    });

    view! { <input node_ref=input_ref type="text"/> }
}
