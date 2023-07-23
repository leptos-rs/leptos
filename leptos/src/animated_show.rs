use crate::Show;
use core::time::Duration;
use leptos::component;
use leptos_dom::{helpers::TimeoutHandle, Fragment, IntoView};
use leptos_macro::view;
use leptos_reactive::{
    create_effect, on_cleanup, signal_prelude::*, store_value, Scope,
    StoredValue,
};

/// A component that will show its children when the `when` condition is `true`.
/// Additionally, you need to specify a `hide_delay`. If the `when` condition changes to `false`,
/// the unmounting of the children will be delayed by the specified Duration.
/// If you provide the optional `show_class` and `hide_class`, you can create very easy mount /
/// unmount animations.
///
/// ```rust
/// # use core::time::Duration;
/// # use leptos::*;
/// # #[component]
/// # pub fn App(cx: Scope) -> impl IntoView {
/// let show = create_rw_signal(cx, false);
///
/// view! { cx,
///     <div
///         class="hover-me"
///         on:mouseenter=move |_| show.set(true)
///         on:mouseleave=move |_| show.set(false)
///     >
///         "Hover Me"
///     </div>
///
///     <AnimatedShow
///        when=show
///        show_class="fade-in-1000"
///        hide_class="fade-out-1000"
///        hide_delay=Duration::from_millis(1000)
///     >
///        <div class="here-i-am">
///            "Here I Am!"
///        </div>
///     </AnimatedShow>
/// }
/// # }
/// ```
#[cfg_attr(
    any(debug_assertions, feature = "ssr"),
    tracing::instrument(level = "info", skip_all)
)]
#[component]
pub fn AnimatedShow(
    /// The scope the component is running in
    cx: Scope,
    /// The components Show wraps
    children: Box<dyn Fn(Scope) -> Fragment>,
    /// If the component should show or not
    #[prop(into)]
    when: MaybeSignal<bool>,
    /// Optional CSS class to apply if `when == true`
    #[prop(optional)]
    show_class: &'static str,
    /// Optional CSS class to apply if `when == false`
    #[prop(optional)]
    hide_class: &'static str,
    /// The timeout after which the component will be unmounted if `when == false`
    hide_delay: Duration,
) -> impl IntoView {
    let handle: StoredValue<Option<TimeoutHandle>> = store_value(cx, None);
    let cls = create_rw_signal(
        cx,
        if when.get_untracked() {
            show_class
        } else {
            hide_class
        },
    );
    let show = create_rw_signal(cx, when.get_untracked());

    create_effect(cx, move |_| {
        if when.get() {
            // clear any possibly active timer
            if let Some(h) = handle.get_value() {
                h.clear();
            }

            cls.set(show_class);
            show.set(true);
        } else {
            cls.set(hide_class);

            let h = leptos_dom::helpers::set_timeout_with_handle(
                move || show.set(false),
                hide_delay,
            )
            .expect("set timeout in AnimatedShow");
            handle.set_value(Some(h));
        }
    });

    on_cleanup(cx, move || {
        if let Some(h) = handle.get_value() {
            h.clear();
        }
    });

    view! { cx,
        <Show when=move || show.get() fallback=|_| ()>
            <div class=move || cls.get()>{children(cx)}</div>
        </Show>
    }
}
