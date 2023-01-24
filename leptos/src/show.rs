use leptos::component;
use leptos_dom::{Fragment, IntoView};
use leptos_reactive::Scope;
use once_cell::sync::Lazy;

/// A component that will show it's children when the passed in closure is True, and show the fallback
/// when the closure is false
#[component]
pub fn Show<F, W, IV>(
    /// The scope the component is running in
    cx: Scope,
    /// The components Show wraps
    children: Box<dyn FnOnce(Scope) -> Fragment>,
    /// A closure that returns a bool that determines whether this thing runs
    when: W,
    /// A closure that returns what gets rendered if the when statement is false
    fallback: F,
) -> impl IntoView
where
    W: Fn() -> bool + 'static,
    F: Fn(Scope) -> IV + 'static,
    IV: IntoView,
{
    // now you don't render until `when` is actually true
    let children = Lazy::new(move || children(cx).into_view(cx));
    let fallback = Lazy::new(move || fallback(cx).into_view(cx));

    move || match when() {
        true => children.clone(),
        false => fallback.clone(),
    }
}
