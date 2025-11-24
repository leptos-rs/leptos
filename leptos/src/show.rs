use crate::{
    children::{TypedChildrenFn, ViewFn},
    IntoView,
};
use leptos_macro::component;
use reactive_graph::{computed::ArcMemo, traits::Get};
use tachys::either::Either;

/// Shows its children whenever the condition `when` prop is `true`.
/// Otherwise it renders the `fallback` prop, which defaults to the empty view.
///
/// The prop `when` can be a closure that returns a bool, a signal of type bool, or a boolean value.
///
/// ## Usage
///
/// ```
/// # use leptos::prelude::*;
/// #
/// # #[component]
/// # pub fn Demo() -> impl IntoView {
/// let (condition, set_condition) = signal(true);
///
/// view! {
///     <Show when=condition>
///         <p>"Hello, world!"</p>
///     </Show>
/// }
/// # }
/// ```
///
/// Or with a closure as the `when` condition:
///
/// ```
/// # use leptos::prelude::*;
/// #
/// # #[component]
/// # pub fn Demo() -> impl IntoView {
/// let (condition, set_condition) = signal(true);
///
/// view! {
///     <Show when=move || condition.get()>
///         <p>"Hello, world!"</p>
///     </Show>
/// }
/// # }
/// ```
#[component]
pub fn Show<W, C>(
    /// The children will be shown whenever the condition in the `when` closure returns `true`.
    children: TypedChildrenFn<C>,
    /// A closure that returns a bool that determines whether this thing runs
    when: W,
    /// A closure that returns what gets rendered if the when statement is false. By default this is the empty view.
    #[prop(optional, into)]
    fallback: ViewFn,
) -> impl IntoView
where
    W: Fn() -> bool + Send + Sync + 'static,
    C: IntoView + 'static,
{
    let memoized_when = ArcMemo::new(move |_| when());
    let children = children.into_inner();

    move || match memoized_when.get() {
        true => Either::Left(children()),
        false => Either::Right(fallback.run()),
    }
}
