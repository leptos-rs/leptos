use leptos_dom::Element;
use leptos_macro::*;
use leptos_reactive::{Memo, Scope};
use std::fmt::Debug;
use std::hash::Hash;

use crate as leptos;
use crate::map::map_keyed;

/// Properties for the [For](crate::For) component.
#[derive(Props)]
pub struct ForProps<E, T, G, I, K>
where
    E: Fn() -> Vec<T>,
    G: Fn(Scope, &T) -> Element,
    I: Fn(&T) -> K,
    K: Eq + Hash,
    T: Eq + 'static,
{
    pub each: E,
    pub key: I,
    pub children: Box<dyn Fn() -> Vec<G>>,
}

/// Iterates over children and displays them, keyed by `PartialEq`.
///
/// This is much more efficient than naively iterating over nodes with `.iter().map(|n| view! { cx,  ... })...`,
/// as it avoids re-creating DOM nodes that are not being changed.
#[allow(non_snake_case)]
pub fn For<E, T, G, I, K>(cx: Scope, props: ForProps<E, T, G, I, K>) -> Memo<Vec<Element>>
//-> impl FnMut() -> Vec<Element>
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
