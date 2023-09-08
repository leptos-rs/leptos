use leptos::{SignalWrite, *};
use std::cell::{Ref, RefMut};

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
    let (value, set_value) = create_signal(initial_value);

    let something: Ref<'_, i32> = value.read();

    spawn_local(async move {
        let mut something_else: RefMut<'_, i32> = set_value.write();
        async {}.await;
        *something_else = 30;
    });

    view! {
        <div>
            <button on:click=move |_| set_value(0)>"Clear"</button>
            <button on:click=move |_| *set_value.write() -= step>"-1"</button>
            <span>"Value: " {value} "!"</span>
            <button on:click=move |_| *set_value.write() += step>"+1"</button>
        </div>
    }
}
