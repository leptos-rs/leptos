use crate::{children::ViewFn, IntoView};
use leptos_macro::component;
use reactive_graph::{traits::Get, wrappers::read::Signal};
use tachys::either::Either;

/// Like `<Show>` but for `Option`. This is a shortcut for
///
/// ```ignore
/// value.map(|value| {
///     view! { ... }
/// })
/// ```
///
/// If you specify a `fallback` it is equvalent to
///
/// ```ignore
/// value
///     .map(
///         |value| children(value),
///     )
///     .unwrap_or_else(fallback)
/// ```
///
/// ## Example
///
/// ```
/// # use leptos::prelude::*;
/// #
/// # #[component]
/// # pub fn Example() -> impl IntoView {
/// let (opt_value, set_opt_value) = signal(None::<i32>);
///
/// view! {
///     <ShowLet some=opt_value let:value>
///         "We have a value: " {value}
///     </ShowLet>
/// }
/// # }
/// ```
///
/// You can also specify a fallback:
/// ```
/// # use leptos::prelude::*;
/// #
/// # #[component]
/// # pub fn Example() -> impl IntoView {
/// let (opt_value, set_opt_value) = signal(None::<i32>);
///
/// view! {
///     <ShowLet some=opt_value let:value fallback=|| "Got nothing">
///         "We have a value: " {value}
///     </ShowLet>
/// }
/// # }
/// ```
///
/// In addition to signals you can also use a closure that returns an `Option`:
///
/// ```
/// # use leptos::prelude::*;
/// #
/// # #[component]
/// # pub fn Example() -> impl IntoView {
/// let (opt_value, set_opt_value) = signal(None::<i32>);
///
/// view! {
///     <ShowLet some=move || opt_value.get().map(|v| v * 2) let:value>
///         "We have a value: " {value}
///     </ShowLet>
/// }
/// # }
/// ```
#[component]
pub fn ShowLet<T, ChFn, V>(
    /// The children will be shown whenever `value` is `Some`.
    ///
    /// They take the inner value as an argument. Use `let:` to bind the value to a variable.
    children: ChFn,

    /// A signal of type `Option` or a closure that returns an `Option`.
    /// If the value is `Some`, the children will be shown.
    /// Otherwise the fallback will be shown, if present.
    #[prop(into)]
    some: Signal<Option<T>>,

    /// A closure that returns what gets rendered when the value is `None`.
    /// By default this is the empty view.
    ///
    /// You can think of it as the closure inside `.unwrap_or_else(|| fallback())`.
    #[prop(optional, into)]
    fallback: ViewFn,
) -> impl IntoView
where
    ChFn: Fn(T) -> V + Send + Clone + 'static,
    V: IntoView + 'static,
    T: Clone + Send + Sync + 'static,
{
    move || {
        let children = children.clone();
        let fallback = fallback.clone();

        some.get()
            .map(move |t| Either::Left(children(t)))
            .unwrap_or_else(move || Either::Right(fallback.run()))
    }
}
