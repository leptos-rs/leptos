use leptos::component;
use leptos_dom::{Fragment, IntoView};
use leptos_reactive::{create_memo, signal_prelude::*};

/// A component that will show its children when the `when` condition is `true`,
/// and show the fallback when it is `false`, without rerendering every time
/// the condition changes.
///
/// *Note*: Because of the nature of generic arguments, it’s not really possible
/// to make the `fallback` optional. If you want an empty fallback state—in other
/// words, if you want to show the children if `when` is true and noting otherwise—use
/// `fallback=|_| ()` (i.e., a fallback function that returns the unit type `()`).
///
/// ```rust
/// # use leptos_reactive::*;
/// # use leptos_macro::*;
/// # use leptos_dom::*; use leptos::*;
/// # run_scope(create_runtime(), |cx| {
/// let (value, set_value) = create_signal(0);
///
/// view! {
///   <Show
///     when=move || value.get() < 5
///     fallback=|| view! {  "Big number!" }
///   >
///     "Small number!"
///   </Show>
/// }
/// # });
/// ```
#[cfg_attr(
    any(debug_assertions, feature = "ssr"),
    tracing::instrument(level = "info", skip_all)
)]
#[component]
pub fn Show<F, W, IV>(
    /// The scope the component is running in

    /// The components Show wraps
    children: Box<dyn Fn() -> Fragment>,
    /// A closure that returns a bool that determines whether this thing runs
    when: W,
    /// A closure that returns what gets rendered if the when statement is false
    fallback: F,
) -> impl IntoView
where
    W: Fn() -> bool + 'static,
    F: Fn() -> IV + 'static,
    IV: IntoView,
{
    let memoized_when = create_memo(move |_| when());

    move || match memoized_when.get() {
        true => children().into_view(),
        false => fallback().into_view(),
    }
}
