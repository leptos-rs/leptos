use leptos::*;

fn update_counter_bg(mut value: i32, step: i32, sig: WriteSignal<i32>) {
    sig.set(value);
    value += step;
    if value < 1000 {
    leptos::set_timeout(
        move || {
            update_counter_bg(value, step, sig);
        },
        std::time::Duration::from_millis(10),
    );
    }
}


#[component]
pub fn SimpleCounter(
    cx: Scope,
    initial_value: i32,
    step: i32,
) -> impl IntoView {
    let (value, set_value) = create_signal(cx, initial_value);

    // update the value signal periodically
    update_counter_bg(initial_value, step, set_value);

    view! { cx,
        <div>
            <div>
                <button on:click=move |_| set_value(0)>"Clear"</button>
                <button on:click=move |_| set_value.update(|value| *value -= step)>"-1"</button>
                <span>"Value: " {value} "!"</span>
                <button on:click=move |_| set_value.update(|value| *value += step)>"+1"</button>
            </div>
            <Show when={move || value() % 2 == 0} fallback=|_| ()>
                <For each={|| vec![1, 2, 3]} key=|key| *key view={move |cx, k| {
                    view! {
                        cx,
                        <article>{k}</article>
                    }
                }}/>
            </Show>
        </div>
    }
}