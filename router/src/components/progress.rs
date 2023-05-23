use leptos::{leptos_dom::helpers::IntervalHandle, *};

/// A visible indicator that the router is in the process of navigating
/// to another route.
///
/// This is used when `<Router set_is_routing>` has been provided, to
/// provide some visual indicator that the page is currently loading
/// async data, so that it is does not appear to have frozen. It can be
/// styled independently.
#[component]
pub fn RoutingProgress(
    cx: Scope,
    /// Whether the router is currently loading the new page.
    #[prop(into)]
    is_routing: Signal<bool>,
    /// The maximum expected time for loading, which is used to
    /// calibrate the animation process.
    #[prop(optional, into)]
    max_time: std::time::Duration,
    /// The time to show the full progress bar after page has loaded, before hiding it. (Defaults to 100ms.)
    #[prop(default = std::time::Duration::from_millis(250))]
    before_hiding: std::time::Duration,
    /// CSS classes to be applied to the `<progress>`.
    #[prop(optional, into)]
    class: String,
) -> impl IntoView {
    const INCREMENT_EVERY_MS: f32 = 5.0;
    let expected_increments =
        max_time.as_secs_f32() / (INCREMENT_EVERY_MS / 1000.0);
    let percent_per_increment = 100.0 / expected_increments;

    let (is_showing, set_is_showing) = create_signal(cx, false);
    let (progress, set_progress) = create_signal(cx, 0.0);

    create_effect(cx, move |prev: Option<Option<IntervalHandle>>| {
        if is_routing.get() && !is_showing.get() {
            set_is_showing.set(true);
            set_interval_with_handle(
                move || {
                    set_progress.update(|n| *n += percent_per_increment);
                },
                std::time::Duration::from_millis(INCREMENT_EVERY_MS as u64),
            )
            .ok()
        } else if is_routing.get() && is_showing.get() {
            set_progress.set(0.0);
            prev?
        } else {
            set_progress.set(100.0);
            set_timeout(
                move || {
                    set_progress.set(0.0);
                    set_is_showing.set(false);
                },
                before_hiding,
            );
            if let Some(Some(interval)) = prev {
                interval.clear();
            }
            None
        }
    });

    view! { cx,
        <Show when=move || is_showing.get() fallback=|_| ()>
            <progress class=class.clone() min="0" max="100" value=move || progress.get()/>
        </Show>
    }
}
