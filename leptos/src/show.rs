use crate::{
    children::{TypedChildrenFn, ViewFn},
    prelude::{FunctionMarker, SignalMarker},
    IntoView,
};
use leptos_macro::component;
use reactive_graph::{computed::ArcMemo, traits::Get};
use std::{marker::PhantomData, sync::Arc};
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
pub fn Show<M, C>(
    /// The children will be shown whenever the condition in the `when` closure returns `true`.
    children: TypedChildrenFn<C>,
    /// When true the children are shown, otherwise the fallback.
    /// It accepts a closure that returns a boolean value as well as a boolean signal or plain boolean value.
    when: impl IntoCondition<M>,
    /// A closure that returns what gets rendered if the when statement is false. By default this is the empty view.
    #[prop(optional, into)]
    fallback: ViewFn,

    /// Marker for generic parameters. Ignore this.
    #[prop(optional)]
    _marker: PhantomData<M>,
) -> impl IntoView
where
    C: IntoView + 'static,
{
    let when = when.into_condition();
    let memoized_when = ArcMemo::new(move |_| when.run());
    let children = children.into_inner();

    move || match memoized_when.get() {
        true => Either::Left(children()),
        false => Either::Right(fallback.run()),
    }
}

/// A closure that returns a bool. Can be converted from a closure, a signal, or a boolean value.
pub struct Condition(Arc<dyn Fn() -> bool + Send + Sync + 'static>);

impl Condition {
    /// Evaluates the condition and returns its result.
    pub fn run(&self) -> bool {
        (self.0)()
    }
}

/// Trait to convert various types into a `Condition`.
/// Implemented for closures, signals, and boolean values.
pub trait IntoCondition<M> {
    /// Does the conversion
    fn into_condition(self) -> Condition;
}

impl<S> IntoCondition<SignalMarker> for S
where
    S: Get<Value = bool> + Send + Sync + 'static,
{
    fn into_condition(self) -> Condition {
        Condition(Arc::new(move || self.get()))
    }
}

impl<F> IntoCondition<FunctionMarker> for F
where
    F: Fn() -> bool + Send + Sync + 'static,
{
    fn into_condition(self) -> Condition {
        Condition(Arc::new(self))
    }
}

impl IntoCondition<Condition> for Condition {
    fn into_condition(self) -> Condition {
        self
    }
}
