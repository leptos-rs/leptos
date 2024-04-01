use leptos::{component, ChildrenFn, ViewFn};
use leptos_dom::IntoView;
use leptos_reactive::signal_prelude::*;

/// A component that will show its children when the `when` condition is `true`,
/// and show the fallback when it is `false`, without rerendering every time
/// the condition changes.
///
/// The fallback prop is optional and defaults to rendering nothing.
///
/// ```rust
/// # use leptos_reactive::*;
/// # use leptos_macro::*;
/// # use leptos_dom::*; use leptos::*;
/// # let runtime = create_runtime();
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
/// # ;
/// # runtime.dispose();
/// ```
#[cfg_attr(
    any(debug_assertions, feature = "ssr"),
    tracing::instrument(level = "trace", skip_all)
)]
#[component]
pub fn Show<W>(
    /// The children will be shown whenever the condition in the `when` closure returns `true`.
    children: ChildrenFn,
    /// A closure that returns a bool that determines whether this thing runs
    when: W,
    /// A closure that returns what gets rendered if the when statement is false. By default this is the empty view.
    #[prop(optional, into)]
    fallback: ViewFn,
) -> impl IntoView
where
    W: Fn() -> bool + 'static,
{
    let memoized_when = create_memo(move |_| when());

    move || match memoized_when.get() {
        true => children().into_view(),
        false => fallback.run(),
    }
}
