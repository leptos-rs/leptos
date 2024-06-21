use leptos::prelude::*;
use leptos_router::hooks::query_signal;

/// A simple counter component.
///
/// You can use doc comments like this to document your component.
#[component]
pub fn SimpleQueryCounter() -> impl IntoView {
    let (count, set_count) = query_signal::<i32>("count");
    let clear = move |_| set_count.set(None);
    let decrement = move |_| set_count.set(Some(count.get().unwrap_or(0) - 1));
    let increment = move |_| set_count.set(Some(count.get().unwrap_or(0) + 1));

    let (msg, set_msg) = query_signal::<String>("message");
    let update_msg = move |ev| {
        let new_msg = event_target_value(&ev);
        if new_msg.is_empty() {
            set_msg.set(None);
        } else {
            set_msg.set(Some(new_msg));
        }
    };

    view! {
        <div>
            <button on:click=clear>"Clear"</button>
            <button on:click=decrement>"-1"</button>
            <span>"Value: " {move || count.get().unwrap_or(0)} "!"</span>
            <button on:click=increment>"+1"</button>

            <br />

            <input
                prop:value=move || msg.get().unwrap_or_default()
                on:input=update_msg
            />
        </div>
    }
}
