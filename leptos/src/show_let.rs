use crate::{children::ViewFn, IntoView};
use leptos_macro::component;
use reactive_graph::traits::Get;
use std::{marker::PhantomData, sync::Arc};
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
pub fn ShowLet<T, ChFn, V, M>(
    /// The children will be shown whenever `value` is `Some`.
    ///
    /// They take the inner value as an argument. Use `let:` to bind the value to a variable.
    children: ChFn,

    /// A signal of type `Option` or a closure that returns an `Option`.
    /// If the value is `Some`, the children will be shown.
    /// Otherwise the fallback will be shown, if present.
    some: impl IntoOptionGetter<T, M>,

    /// A closure that returns what gets rendered when the value is `None`.
    /// By default this is the empty view.
    ///
    /// You can think of it as the closure inside `.unwrap_or_else(|| fallback())`.
    #[prop(optional, into)]
    fallback: ViewFn,

    /// Marker for generic parameters. Ignore this.
    #[prop(optional)]
    _marker: PhantomData<(T, M)>,
) -> impl IntoView
where
    ChFn: Fn(T) -> V + Send + Clone + 'static,
    V: IntoView + 'static,
    T: 'static,
{
    let getter = some.into_option_getter();

    move || {
        let children = children.clone();
        let fallback = fallback.clone();

        getter
            .run()
            .map(move |t| Either::Left(children(t)))
            .unwrap_or_else(move || Either::Right(fallback.run()))
    }
}

/// Servers as a wrapper for both, an `Option` signal or a closure that returns an `Option`.
pub struct OptionGetter<T>(Arc<dyn Fn() -> Option<T> + Send + Sync + 'static>);

impl<T> Clone for OptionGetter<T> {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

impl<T> OptionGetter<T> {
    /// Runs the getter and returns the result.
    pub fn run(&self) -> Option<T> {
        (self.0)()
    }
}

/// Conversion trait for creating an `OptionGetter` from a closure or a signal.
pub trait IntoOptionGetter<T, M> {
    /// Converts the given value into an `OptionGetter`.
    fn into_option_getter(self) -> OptionGetter<T>;
}

/// Marker type for creating an `OptionGetter` from a closure.
/// Used so that the compiler doesn't complain about double implementations of the trait `IntoOptionGetter`.
pub struct FunctionMarker;

impl<T, F> IntoOptionGetter<T, FunctionMarker> for F
where
    F: Fn() -> Option<T> + Send + Sync + 'static,
{
    fn into_option_getter(self) -> OptionGetter<T> {
        OptionGetter(Arc::new(self))
    }
}

/// Marker type for creating an `OptionGetter` from a signal.
/// Used so that the compiler doesn't complain about double implementations of the trait `IntoOptionGetter`.
pub struct SignalMarker;

impl<T, S> IntoOptionGetter<T, SignalMarker> for S
where
    S: Get<Value = Option<T>> + Clone + Send + Sync + 'static,
{
    fn into_option_getter(self) -> OptionGetter<T> {
        let cloned = self.clone();
        OptionGetter(Arc::new(move || cloned.get()))
    }
}
