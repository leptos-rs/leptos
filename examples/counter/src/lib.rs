use leptos::*;

pub fn simple_counter(cx: Scope) -> web_sys::Element {
    let (value, set_value) = create_signal(cx, 0);
    log::debug!("ok");

    view! {
        <div>
            <button on:click=move |_| set_value(|value| *value -= 1)>"-1"</button>
            <span>"Value: " {move || value().to_string()}</span>
            <button on:click=move |_| set_value(|value| *value += 1)>"+1"</button>
        </div>
    }
}
