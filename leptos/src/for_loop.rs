use leptos_dom::IntoView;
use leptos_macro::component;
use std::hash::Hash;

/// Iterates over children and displays them, keyed by the `key` function given.
///
/// This is much more efficient than naively iterating over nodes with `.iter().map(|n| view! { ... })...`,
/// as it avoids re-creating DOM nodes that are not being changed.
///
/// ```
/// # use leptos::*;
///
/// #[derive(Copy, Clone, Debug, PartialEq, Eq)]
/// struct Counter {
///   id: usize,
///   count: RwSignal<i32>
/// }
///
/// #[component]
/// fn Counters() -> impl IntoView {
///   let (counters, set_counters) = create_signal::<Vec<Counter>>(vec![]);
///
///   view! {
///     <div>
///       <For
///         // a function that returns the items we're iterating over; a signal is fine
///         each=move || counters.get()
///         // a unique key for each item
///         key=|counter| counter.id
///         // renders each item to a view
///         children=move |counter: Counter| {
///           view! {
///             <button>"Value: " {move || counter.count.get()}</button>
///           }
///         }
///       />
///     </div>
///   }
/// }
/// ```
#[cfg_attr(
    any(debug_assertions, feature = "ssr"),
    tracing::instrument(level = "trace", skip_all)
)]
#[component(transparent)]
pub fn For<IF, I, T, EF, N, KF, K>(
    /// Items over which the component should iterate.
    each: IF,
    /// A key function that will be applied to each item.
    key: KF,
    /// A function that takes the item, and returns the view that will be displayed for each item.
    ///
    /// ## Syntax
    /// This can be passed directly in the `view` children of the `<For/>` by using the
    /// `let:` syntax to specify the name for the data variable passed in the argument.
    ///
    /// ```rust
    /// # use leptos::*;
    /// # if false {
    /// let (data, set_data) = create_signal(vec![0, 1, 2]);
    /// view! {
    ///     <For
    ///         each=move || data.get()
    ///         key=|n| *n
    ///         // stores the item in each row in a variable named `data`
    ///         let:data
    ///     >
    ///         <p>{data}</p>
    ///     </For>
    /// }
    /// # ;
    /// # }
    /// ```
    /// is the same as
    ///  ```rust
    /// # use leptos::*;
    /// # if false {
    /// let (data, set_data) = create_signal(vec![0, 1, 2]);
    /// view! {
    ///     <For
    ///         each=move || data.get()
    ///         key=|n| *n
    ///         children=|data| view! { <p>{data}</p> }
    ///     />
    /// }
    /// # ;
    /// # }
    /// ```
    children: EF,
) -> impl IntoView
where
    IF: Fn() -> I + 'static,
    I: IntoIterator<Item = T>,
    EF: Fn(T) -> N + 'static,
    N: IntoView + 'static,
    KF: Fn(&T) -> K + 'static,
    K: Eq + Hash + 'static,
    T: 'static,
{
    leptos_dom::Each::new(each, key, children).into_view()
}
