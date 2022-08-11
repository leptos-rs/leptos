use leptos_dom::Element;
use leptos_macro::*;
use leptos_reactive::{ReadSignal, Scope};
use std::fmt::Debug;
use std::hash::Hash;

use crate as leptos;
use crate::map::map_keyed;

/// Properties for the [For](crate::For) component.
#[derive(Props)]
pub struct ForProps<T, G, I, K>
where
    G: Fn(Scope, &T) -> Element,
    I: Fn(&T) -> K,
    K: Eq + Hash,
    T: Eq + Clone + 'static,
{
    pub each: ReadSignal<Vec<T>>,
    pub key: I,
    pub children: G,
}

/// Iterates over children and displays them, keyed by `PartialEq`. If you want to provide your
/// own key function, use [Index] instead.
///
/// This is much more efficient than naively iterating over nodes with `.iter().map(|n| view! { ... })...`,
/// as it avoids re-creating DOM nodes that are not being changed.
#[allow(non_snake_case)]
pub fn For<T, G, I, K>(cx: Scope, props: ForProps<T, G, I, K>) -> ReadSignal<Vec<Element>>
where
    G: Fn(Scope, &T) -> Element + 'static,
    I: Fn(&T) -> K + 'static,
    K: Eq + Hash,
    T: Eq + Clone + Debug + 'static,
{
    map_keyed(cx, props.each, props.children, props.key).clone()
}
