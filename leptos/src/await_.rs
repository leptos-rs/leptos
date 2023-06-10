use crate::Suspense;
use leptos_dom::IntoView;
use leptos_macro::{component, view};
use leptos_reactive::{
    create_blocking_resource, create_resource, store_value, Scope, Serializable,
};

#[component]
/// Allows you to inline the data loading for an `async` block or
/// server function directly into your view. This is the equivalent of combining a
/// [`create_resource`] that only loads once (i.e., with a source signal `|| ()`) with
/// a [`Suspense`] with no `fallback`.
///
/// Adding `bind:{variable name}` to the props makes the data available in the children
/// that variable name, when resolved.
/// ```
/// # use leptos_reactive::*;
/// # use leptos_macro::*;
/// # use leptos_dom::*; use leptos::*;
/// # if false {
/// # run_scope(create_runtime(), |cx| {
/// async fn fetch_monkeys(monkey: i32) -> i32 {
///     // do some expensive work
///     3
/// }
///
/// view! { cx,
///     <Await
///         future=|cx| fetch_monkeys(3)
///         bind:data
///     >
///         <p>{*data} " little monkeys, jumping on the bed."</p>
///     </Await>
/// }
/// # ;
/// # });
/// # }
/// ```
pub fn Await<T, Fut, FF, VF, V>(
    cx: Scope,
    /// A function that takes a [`Scope`] and returns the [`Future`](std::future::Future) that
    /// will the component will `.await` before rendering.
    future: FF,
    /// If `true`, the component will use [`create_blocking_resource`], preventing
    /// the HTML stream from returning anything before `future` has resolved.
    #[prop(optional)]
    blocking: bool,
    /// A function that takes a [`Scope`] and a reference to the resolved data from the `future`
    /// renders a view.
    ///
    /// ## Syntax
    /// This can be passed in the `view` children of the `<Await/>` by using the
    /// `bind:` syntax to specify the name for the data variable.
    ///
    /// ```rust
    /// # use leptos::*;
    /// # if false {
    /// # run_scope(create_runtime(), |cx| {
    /// # async fn fetch_monkeys(monkey: i32) -> i32 {
    /// #    3
    /// # }
    /// view! { cx,
    ///     <Await
    ///         future=|cx| fetch_monkeys(3)
    ///         bind:data
    ///     >
    ///         <p>{*data} " little monkeys, jumping on the bed."</p>
    ///     </Await>
    /// }
    /// # ;
    /// # })
    /// # }
    /// ```
    /// is the same as
    ///  ```rust
    /// # use leptos::*;
    /// # if false {
    /// # run_scope(create_runtime(), |cx| {
    /// # async fn fetch_monkeys(monkey: i32) -> i32 {
    /// #    3
    /// # }
    /// view! { cx,
    ///     <Await
    ///         future=|cx| fetch_monkeys(3)
    ///         children=|cx, data| view! { cx,
    ///           <p>{*data} " little monkeys, jumping on the bed."</p>
    ///         }
    ///     />
    /// }
    /// # ;
    /// # })
    /// # }
    /// ```
    children: VF,
) -> impl IntoView
where
    Fut: std::future::Future<Output = T> + 'static,
    FF: Fn(Scope) -> Fut + 'static,
    V: IntoView,
    VF: Fn(Scope, &T) -> V + 'static,
    T: Serializable + 'static,
{
    let res = if blocking {
        create_blocking_resource(cx, || (), move |_| future(cx))
    } else {
        create_resource(cx, || (), move |_| future(cx))
    };
    let view = store_value(cx, children);
    view! { cx,
        <Suspense fallback=|| ()>
            {move || res.with(cx, |data| view.with_value(|view| view(cx, data)))}
        </Suspense>
    }
}
