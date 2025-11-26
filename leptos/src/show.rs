use crate::{
    children::{TypedChildrenFn, ViewFn},
    IntoView,
};
use leptos_macro::component;
use reactive_graph::{computed::ArcMemo, traits::Get, wrappers::read::Signal};
use tachys::either::Either;

/// Includes it's children in the DOM if and only if `when` is `true`.
///
/// ## Example
///
/// ```
/// # use leptos::prelude::*;
/// #
/// # #[component]
/// # pub fn Example() -> impl IntoView {
/// let (value, set_value) = signal(true);
///
/// view! {
///     <Show when=value>
///         "Value is true"
///     </Show>
/// }
/// # }
/// ```
///
/// You can also specify a `fallback` view to render if `when` is `false`:
///
/// ```
/// # use leptos::prelude::*;
/// #
/// # #[component]
/// # pub fn Example() -> impl IntoView {
/// let (value, set_value) = signal(true);
///
/// view! {
///     <Show when=value fallback=|| "False">
///         "True"
///     </Show>
/// }
/// # }
/// ```
///
/// In addition to signals you can also use a closure that returns an `bool`:
///
/// ```
/// # use leptos::prelude::*;
/// #
/// # #[component]
/// # pub fn Example() -> impl IntoView {
/// let (value, set_value) = signal(3);
///
/// view! {
///     <Show when=move || { value.get() > 0 }>
///         "Value greater than zero"
///     </Show>
/// }
/// # }
/// ```
#[component]
pub fn Show<C>(
    /// The children will be shown whenever the condition in the `when` closure returns `true`.
    children: TypedChildrenFn<C>,
    /// A signal of a bool that determines whether this thing runs. This also accepts a closure that returns a bool.
    #[prop(into)]
    when: Signal<bool>,
    /// A closure that returns what gets rendered if the when statement is false. By default this is the empty view.
    #[prop(optional, into)]
    fallback: ViewFn,
) -> impl IntoView
where
    C: IntoView + 'static,
{
    let memoized_when = ArcMemo::new(move |_| when.get());
    let children = children.into_inner();

    move || match memoized_when.get() {
        true => Either::Left(children()),
        false => Either::Right(fallback.run()),
    }
}
