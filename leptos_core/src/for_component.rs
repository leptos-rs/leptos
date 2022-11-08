use leptos_dom::Element;
use leptos_macro::*;
use leptos_reactive::{Memo, Scope};
use std::fmt::Debug;
use std::hash::Hash;

use crate as leptos;
use crate::map::map_keyed;

/// Properties for the [For](crate::For) component, a keyed list.
#[derive(Props)]
pub struct ForProps<E, T, G, I, K>
where
    E: Fn() -> Vec<T>,
    G: Fn(Scope, &T) -> Element,
    I: Fn(&T) -> K,
    K: Eq + Hash,
    T: Eq + 'static,
{
    /// Items over which the component should iterate.
    pub each: E,
    /// A key function that will be applied to each item
    pub key: I,
    /// Should provide a single child function, which takes
    pub children: Box<dyn Fn() -> Vec<G>>,
}

/// Iterates over children and displays them, keyed by the `key` function given.
///
/// This is much more efficient than naively iterating over nodes with `.iter().map(|n| view! { cx,  ... })...`,
/// as it avoids re-creating DOM nodes that are not being changed.
///
/// ```
/// # use leptos_reactive::*;
/// # use leptos_macro::*;
/// # use leptos_core::*;
/// # use leptos_dom::*;
///
/// #[derive(Copy, Clone, Debug, PartialEq, Eq)]
/// struct Counter {
///   id: usize,
///   count: RwSignal<i32>
/// }
///
/// fn Counters(cx: Scope) -> Element {
///   let (counters, set_counters) = create_signal::<Vec<Counter>>(cx, vec![]);
///
///   view! {
///     cx,
///     <div>
///       <For
///         // a function that returns the items we're iterating over; a signal is fine
///         each=counters
///         // a unique key for each item
///         key=|counter| counter.id
///       >
///         {|cx: Scope, counter: &Counter| {
///           let count = counter.count;
///           view! {
///             cx,
///             <button>"Value: " {move || count.get()}</button>
///           }
///         }
///       }
///       </For>
///     </div>
///   }
/// }
/// ```
#[allow(non_snake_case)]
pub fn For<E, T, G, I, K>(cx: Scope, props: ForProps<E, T, G, I, K>) -> Memo<Vec<Element>>
where
    E: Fn() -> Vec<T> + 'static,
    G: Fn(Scope, &T) -> Element + 'static,
    I: Fn(&T) -> K + 'static,
    K: Eq + Hash,
    T: Eq + Debug + 'static,
{
    let map_fn = (props.children)().swap_remove(0);
    map_keyed(cx, props.each, map_fn, props.key)
}
