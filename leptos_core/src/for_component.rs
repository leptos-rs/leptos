use leptos_dom::Element;
use leptos_macro::*;
use leptos_reactive::{create_effect, Memo, ReadSignal, Scope};
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
    T: Eq + Clone + 'static,
{
    pub each: E,
    pub key: I,
    pub children: Vec<G>,
}

/// Iterates over children and displays them, keyed by `PartialEq`. If you want to provide your
/// own key function, use [Index] instead.
///
/// This is much more efficient than naively iterating over nodes with `.iter().map(|n| view! { ... })...`,
/// as it avoids re-creating DOM nodes that are not being changed.
#[allow(non_snake_case)]
pub fn For<E, T, G, I, K>(cx: Scope, mut props: ForProps<E, T, G, I, K>) -> Memo<Vec<Element>>
//-> impl FnMut() -> Vec<Element>
where
    E: Fn() -> Vec<T> + 'static,
    G: Fn(Scope, &T) -> Element + 'static,
    I: Fn(&T) -> K + 'static,
    K: Eq + Hash,
    T: Eq + Clone + Debug + 'static,
{
    let map_fn = props.children.remove(0);
    let m = map_keyed(cx, props.each, map_fn, props.key);
    create_effect(cx, move |_| {
        log::debug!("<For/> component memo len is {}", m.with(|m| m.len()));
    });
    m
}
