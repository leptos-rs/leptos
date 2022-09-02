use leptos::*;

#[component]
pub fn Counters(cx: Scope) -> Element {
    view! {
        <div class="counters">
            <Counter initial_value=1/>
            <Counter initial_value=2/>
        </div>
    }
}

#[component]
pub fn Counter(cx: Scope, initial_value: i32) -> Element {
    let (value, set_value) = create_signal(cx, initial_value);

    create_effect(cx, move |_| log::debug!("value is now {}", value()));

    view! {
        <div>
            <button on:click=move |_| { log::debug!("clicked -1"); set_value(|value| *value -= 1) }>"-1"</button>
            <span>"Value: " {move || value().to_string()} "!"</span>
            <button on:click=move |_| { log::debug!("clicked +1"); set_value(|value| *value += 1) }>"+1"</button>
        </div>
    }
}
