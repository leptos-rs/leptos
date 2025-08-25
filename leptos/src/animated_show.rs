use crate::{
    children::ChildrenFn, component, control_flow::Show, show::IntoCondition,
    IntoView,
};
use core::time::Duration;
use leptos_dom::helpers::TimeoutHandle;
use leptos_macro::view;
use reactive_graph::{
    diagnostics::SpecialNonReactiveZone,
    effect::RenderEffect,
    owner::{on_cleanup, StoredValue},
    signal::RwSignal,
    traits::{Get, GetValue, Set, SetValue},
};
use std::marker::PhantomData;
use tachys::prelude::*;

/// A component that will show its children when the `when` condition is `true`.
/// Additionally, you need to specify a `hide_delay`. If the `when` condition changes to `false`,
/// the unmounting of the children will be delayed by the specified Duration.
/// If you provide the optional `show_class` and `hide_class`, you can create very easy mount /
/// unmount animations.
///
/// ```rust
/// # use core::time::Duration;
/// # use leptos::prelude::*;
/// # #[component]
/// # pub fn App() -> impl IntoView {
/// let show = RwSignal::new(false);
///
/// view! {
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
///
/// Please note, that unlike `Show`, `AnimatedShow` does not support a `fallback` prop.
#[cfg_attr(feature = "tracing", tracing::instrument(level = "trace", skip_all))]
#[component]
pub fn AnimatedShow<M>(
    /// The components Show wraps
    children: ChildrenFn,
    /// When true the children are shown.
    /// It accepts a closure that returns a boolean value as well as a boolean signal or plain boolean value.
    when: impl IntoCondition<M>,
    /// Optional CSS class to apply if `when == true`
    #[prop(optional)]
    show_class: &'static str,
    /// Optional CSS class to apply if `when == false`
    #[prop(optional)]
    hide_class: &'static str,
    /// The timeout after which the component will be unmounted if `when == false`
    hide_delay: Duration,

    /// Marker for generic parameters. Ignore this.
    #[prop(optional)]
    _marker: PhantomData<M>,
) -> impl IntoView {
    let when = when.into_condition();

    // Silence warnings about using signals in non-reactive contexts.
    #[cfg(debug_assertions)]
    let z = SpecialNonReactiveZone::enter();

    let handle: StoredValue<Option<TimeoutHandle>> = StoredValue::new(None);
    let cls = RwSignal::new(if when.run() { show_class } else { hide_class });
    let show = RwSignal::new(when.run());

    #[cfg(debug_assertions)]
    drop(z);

    let eff = RenderEffect::new(move |_| {
        if when.run() {
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

    on_cleanup(move || {
        if let Some(Some(h)) = handle.try_get_value() {
            h.clear();
        }
        drop(eff);
    });

    view! {
        <Show when=show>
            <div class=cls>{children()}</div>
        </Show>
    }
}
