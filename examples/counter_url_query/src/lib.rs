use leptos::*;
use leptos_router::*;

/// A simple counter component.
///
/// You can use doc comments like this to document your component.
#[component]
pub fn SimpleQueryCounter() -> impl IntoView {
    let (count, set_count) = create_query_signal::<i32>("count");
    let clear = move |_| set_count(None);
    let decrement = move |_| set_count(Some(count().unwrap_or(0) - 1));
    let increment = move |_| set_count(Some(count().unwrap_or(0) + 1));

    let (msg, set_msg) = create_query_signal::<String>("message");
    let update_msg = move |ev| {
        let new_msg = event_target_value(&ev);
        if new_msg.is_empty() {
            set_msg(None);
        } else {
            set_msg(Some(new_msg));
        }
    };

    view! {
        <div>
            <button on:click=clear>"Clear"</button>
            <button on:click=decrement>"-1"</button>
            <span>"Value: " {move || count().unwrap_or(0)} "!"</span>
            <button on:click=increment>"+1"</button>

            <br />

            <input
                prop:value=move || msg().unwrap_or_default()
                on:input=update_msg
            />
        </div>
    }
}
