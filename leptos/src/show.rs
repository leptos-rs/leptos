use leptos::{component, ChildrenFn};
use leptos_dom::{IntoView, View};
use leptos_reactive::{create_memo, signal_prelude::*};
use std::rc::Rc;

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
    tracing::instrument(level = "info", skip_all)
)]
#[component]
pub fn Show<W>(
    /// The children will be shown whenever the condition in the `when` closure returns `true`.
    children: ChildrenFn,
    /// A closure that returns a bool that determines whether this thing runs
    when: W,
    /// A closure that returns what gets rendered if the when statement is false. By default this is the empty view.
    #[prop(optional, into)]
    fallback: Fallback,
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

/// New-type wrapper for the fallback view function to enable `#[prop(into, optional)]`.
#[derive(Clone)]
pub struct Fallback {
    function: Rc<dyn Fn() -> View>,
}

impl Default for Fallback {
    fn default() -> Self {
        Self {
            function: Rc::new(|| ().into_view()),
        }
    }
}

impl<F, IV> From<F> for Fallback
where
    F: Fn() -> IV + 'static,
    IV: IntoView,
{
    fn from(value: F) -> Self {
        Self {
            function: Rc::new(move || value().into_view()),
        }
    }
}

impl Fallback {
    /// Execute the wrapped function
    pub fn run(&self) -> View {
        (self.function)()
    }
}
