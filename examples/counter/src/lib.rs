use leptos::*;

pub fn simple_counter(cx: Scope) -> web_sys::Element {
    let (value, set_value) = cx.create_signal(0);

    view! {
        <div>
            <button on:click=move |_| set_value(|value| *value -= 1)>"-1"</button>
            <span>{|| value.get().to_string()}</span>
            <button on:click=move |_| set_value(|value| *value += 1)>"+1"</button>
        </div>
    }
}
