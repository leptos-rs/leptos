use crate::{either::Either, prelude::*};

use super::{Mappable, ViewFnWithParam};

/// Like `<Show>` but for `Option` and `Result`. This is a shortcut for
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
///     <Map value=move || opt_value.get() let:value>
///         "We have a value: " {value}
///     </Map>
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
/// let (res_value, set_res_value) = signal::<Result<i32, String>>(Ok(42));
///
/// view! {
///     <Map value=move || res_value.get() let:value fallback=|err| view! { {format!("Got an error: {err}")} }>
///         "We have an ok value: " {value}
///     </Map>
/// }
/// # }
/// ```
#[component]
pub fn Map<M, T, F, ChFn, V>(
    /// The children will be shown whenever `value` is `Some` or `Ok`.
    children: ChFn,
    /// A closure that returns a `Result` or `Option`.
    /// If the value is `Ok` or `Some`, the children will be shown.
    /// Otherwise the fallback will be shown, if present.
    value: F,
    /// A closure that returns what gets rendered when the value is `None` or `Err`.
    /// By default this is the empty view.
    ///
    /// It can take a single argument. If `value` is an `Option` the argument
    /// will be `()`. In case `value` is a `Result` the argument will be the error value
    /// (you can think of it as the closure inside `.map_err`).
    #[prop(optional, into)]
    fallback: ViewFnWithParam<M::Error>,
) -> impl IntoView
where
    F: Fn() -> M + Send + Sync + 'static,
    M: Mappable<T> + Clone + Send + Sync + 'static,
    M::Error: 'static,
    ChFn: Fn(T) -> V + Send + Clone + 'static,
    V: IntoView + 'static,
{
    move || {
        let children = children.clone();
        let fallback = fallback.clone();

        value()
            .map(move |t| Either::Left(children(t)))
            .unwrap_or_else(move |err| Either::Right(fallback.run(err)))
    }
}
