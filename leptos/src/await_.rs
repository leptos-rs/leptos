use crate::Suspense;
use leptos_dom::IntoView;
use leptos_macro::{component, view};
use leptos_reactive::{
    create_blocking_resource, create_local_resource, create_resource,
    store_value, Serializable,
};

#[component]
/// Allows you to inline the data loading for an `async` block or
/// server function directly into your view. This is the equivalent of combining a
/// [`create_resource`] that only loads once (i.e., with a source signal `|| ()`) with
/// a [`Suspense`] with no `fallback`.
///
/// Adding `let:{variable name}` to the props makes the data available in the children
/// that variable name, when resolved.
/// ```
/// # use leptos_reactive::*;
/// # use leptos_macro::*;
/// # use leptos_dom::*; use leptos::*;
/// # if false {
/// # let runtime = create_runtime();
/// async fn fetch_monkeys(monkey: i32) -> i32 {
///     // do some expensive work
///     3
/// }
///
/// view! {
///     <Await
///         future=|| fetch_monkeys(3)
///         let:data
///     >
///         <p>{*data} " little monkeys, jumping on the bed."</p>
///     </Await>
/// }
/// # ;
/// # runtime.dispose();
/// # }
/// ```
pub fn Await<T, Fut, FF, VF, V>(
    /// A function that returns the [`Future`](std::future::Future) that
    /// will the component will `.await` before rendering.
    future: FF,
    /// If `true`, the component will use [`create_blocking_resource`], preventing
    /// the HTML stream from returning anything before `future` has resolved.
    #[prop(optional)]
    blocking: bool,
    /// If `true`, the component will use [`create_local_resource`], this will
    /// always run on the local system and therefore its result type does not
    /// need to be `Serializable`.
    #[prop(optional)]
    local: bool,
    /// A function that takes a reference to the resolved data from the `future`
    /// renders a view.
    ///
    /// ## Syntax
    /// This can be passed in the `view` children of the `<Await/>` by using the
    /// `let:` syntax to specify the name for the data variable.
    ///
    /// ```rust
    /// # use leptos::*;
    /// # if false {
    /// # let runtime = create_runtime();
    /// # async fn fetch_monkeys(monkey: i32) -> i32 {
    /// #    3
    /// # }
    /// view! {
    ///     <Await
    ///         future=|| fetch_monkeys(3)
    ///         let:data
    ///     >
    ///         <p>{*data} " little monkeys, jumping on the bed."</p>
    ///     </Await>
    /// }
    /// # ;
    /// # runtime.dispose();
    /// # }
    /// ```
    /// is the same as
    ///  ```rust
    /// # use leptos::*;
    /// # if false {
    /// # let runtime = create_runtime();
    /// # async fn fetch_monkeys(monkey: i32) -> i32 {
    /// #    3
    /// # }
    /// view! {
    ///     <Await
    ///         future=|| fetch_monkeys(3)
    ///         children=|data| view! {
    ///           <p>{*data} " little monkeys, jumping on the bed."</p>
    ///         }
    ///     />
    /// }
    /// # ;
    /// # runtime.dispose();
    /// # }
    /// ```
    children: VF,
) -> impl IntoView
where
    Fut: std::future::Future<Output = T> + 'static,
    FF: Fn() -> Fut + 'static,
    V: IntoView,
    VF: Fn(&T) -> V + 'static,
    T: Serializable + 'static,
{
    let res = if blocking {
        create_blocking_resource(|| (), move |_| future())
    } else if local {
        create_local_resource(|| (), move |_| future())
    } else {
        create_resource(|| (), move |_| future())
    };
    let view = store_value(children);

    view! {
        <Suspense fallback=|| ()>
            {move || res.map(|data| view.with_value(|view| view(data)))}
        </Suspense>
    }
}
