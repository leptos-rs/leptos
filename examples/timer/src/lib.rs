use leptos::*;
use wasm_bindgen::{prelude::Closure, JsCast};

/// Timer example, demonstrating the use of `use_interval`.
#[component]
pub fn TimerDemo(cx: Scope) -> impl IntoView {
    // count_a updates with a fixed interval of 1000 ms, whereas count_b has a dynamic
    // update interval.
    let (count_a, set_count_a) = create_signal(cx, 0_i32);
    let (count_b, set_count_b) = create_signal(cx, 0_i32);

    let (interval, set_interval) = create_signal(cx, 1000_i32);

    use_interval(cx, 1000, move || {
        set_count_a.update(|c| *c = *c + 1);
    });
    use_interval(cx, interval, move || {
        set_count_b.update(|c| *c = *c + 1);
    });

    view! { cx,
        <div>
            <div>"Count A (fixed interval of 1000 ms)"</div>
            <div>{count_a}</div>
            <div>"Count B (dynamic interval, currently " {interval} "ms )"</div>
            <div>{count_b}</div>
            <input prop:value=interval on:input=move |ev| {
                if let Ok(value) = event_target_value(&ev).parse::<i32>() {
                    set_interval(value);
                }
            }/>
        </div>
    }
}

pub fn use_interval<T, F>(cx: Scope, interval_millis: T, f: F)
where
    F: Fn() -> () + 'static,
    T: Into<MaybeSignal<i32>> + 'static,
{
    let js_callback: Closure<dyn Fn()> = Closure::new(move || {
        log!("Running timer");
        f();
    });
    let js_callback_clone = js_callback.as_ref().clone();

    let interval_millis = interval_millis.into();

    create_effect(cx, move |_| {
        let window = web_sys::window().unwrap();
        let interval_id = window
            .set_interval_with_callback_and_timeout_and_arguments_0(
                js_callback_clone.unchecked_ref(),
                interval_millis(),
            )
            .expect("Failed set interval");

        on_cleanup(cx, move || {
            // This is needed to clean up the interval itself.
            let window = web_sys::window().unwrap();
            window.clear_interval_with_handle(interval_id);
        })
    });

    on_cleanup(cx, move || {
        // This is needed to keep the closure alive for long enough.
        let _keep_alive = js_callback;
    });
}
