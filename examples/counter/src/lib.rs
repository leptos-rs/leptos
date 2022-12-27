use leptos::*;

/// A simple counter component.
/// 
/// You can document each of the properties passed to a component using the format below.
/// 
/// # Props
/// - **initial_value** [`i32`] - The value the counter should start at.
/// - **step** [`i32`] - The change that should be applied on each step.
#[component]
pub fn SimpleCounter(cx: Scope, initial_value: i32, step: i32) -> web_sys::Element {
    let (value, set_value) = create_signal(cx, initial_value);

    view! { cx,
        <div>
            <button on:click=move |_| set_value(0)>"Clear"</button>
            <button on:click=move |_| set_value.update(|value| *value -= step)>"-1"</button>
            <span>"Value: " {move || value().to_string()} "!"</span>
            <button on:click=move |_| set_value.update(|value| *value += step)>"+1"</button>
        </div>
    }
}
