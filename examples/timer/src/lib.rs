use leptos::{leptos_dom::helpers::IntervalHandle, *};
use std::time::Duration;

/// Timer example, demonstrating the use of `use_interval`.
#[component]
pub fn TimerDemo() -> impl IntoView {
    // count_a updates with a fixed interval of 1000 ms, whereas count_b has a dynamic
    // update interval.
    let (count_a, set_count_a) = create_signal(0_i32);
    let (count_b, set_count_b) = create_signal(0_i32);

    let (interval, set_interval) = create_signal(1000);

    use_interval(1000, move || {
        set_count_a.update(|c| *c += 1);
    });
    use_interval(interval, move || {
        set_count_b.update(|c| *c += 1);
    });

    view! {
        <div>
            <div>"Count A (fixed interval of 1000 ms)"</div>
            <div>{count_a}</div>
            <div>"Count B (dynamic interval, currently " {interval} " ms)"</div>
            <div>{count_b}</div>
            <input prop:value=interval on:input=move |ev| {
                if let Ok(value) = event_target_value(&ev).parse::<u64>() {
                    set_interval.set(value);
                }
            }/>
        </div>
    }
}

/// Hook to wrap the underlying `setInterval` call and make it reactive w.r.t.
/// possible changes of the timer interval.
pub fn use_interval<T, F>(interval_millis: T, f: F)
where
    F: Fn() + Clone + 'static,
    T: Into<MaybeSignal<u64>> + 'static,
{
    let interval_millis = interval_millis.into();
    create_effect(move |prev_handle: Option<IntervalHandle>| {
        // effects get their previous return value as an argument
        // each time the effect runs, it will return the interval handle
        // so if we have a previous one, we cancel it
        if let Some(prev_handle) = prev_handle {
            prev_handle.clear();
        };

        // here, we return the handle
        set_interval_with_handle(
            f.clone(),
            // this is the only reactive access, so this effect will only
            // re-run when the interval changes
            Duration::from_millis(interval_millis.get()),
        )
        .expect("could not create interval")
    });
}
