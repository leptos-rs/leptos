use leptos::*;

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

    view! {
        <div>
            <button on:click=move |_| set_value.set(0)>"Clear"</button>
            <button on:click=move |_| set_value.update(|value| *value -= step)>"-1"</button>
            <span>"Value: " {value} "!"</span>
            <button on:click=move |_| {
                // Test Panic
                //panic!("Test Panic");
                // In order to breakpoint the below, the code needs to be on it's own line
                set_value.update(|value| *value += step)
            }
            >"+1"</button>
        </div>
    }
}
