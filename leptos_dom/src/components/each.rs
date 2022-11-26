// use std::{collections::HashMap, hash::Hash};

// use leptos_reactive::create_effect;

// use crate::{Component, IntoNode};

// #[derive(typed_builder::TypedBuilder)]
// struct EachProps {}

// #[allow(non_snake_case)]
// /// ```html
// /// <ul>
// ///     <!-- <Each> -->
// ///     <!-- <Item> -->
// ///     <li>1</li>
// ///     <!-- </Item> -->
// ///     <!-- <Item> -->
// ///     <li>2</li>
// ///     <!-- </Item> -->
// ///     <!-- </Each> -->
// /// </ul>
// /// ```
// struct Each<IF, I, T, EF, N, KF, K>
// where
//     IF: Fn() -> I + 'static,
//     I: IntoIterator<Item = T>,
//     EF: Fn(T) -> N + 'static,
//     N: IntoNode,
//     KF: Fn(&T) -> K + 'static,
//     K: Eq + Hash + 'static,
//     T: 'static,
// {
//     items_fn: IF,
//     each_fn: EF,
//     key_fn: KF,
// }

// impl<IF, I, T, EF, N, KF, K> Each<IF, I, T, EF, N, KF, K>
// where
//     IF: Fn() -> I + 'static,
//     I: IntoIterator<Item = T>,
//     EF: Fn(T) -> N + 'static,
//     N: IntoNode,
//     KF: Fn(&T) -> K,
//     K: Eq + Hash + 'static,
//     T: 'static,
// {
//     pub fn new(items_fn: IF, each_fn: EF, key_fn: KF) -> Self {
//         Self {
//             items_fn,
//             each_fn,
//             key_fn,
//         }
//     }
// }

// impl<IF, I, T, EF, N, KF, K> IntoNode for Each<IF, I, T, EF, N, KF, K>
// where
//     IF: Fn() -> I + 'static,
//     I: IntoIterator<Item = T>,
//     EF: Fn(T) -> N + 'staticedg
//     ,
//     N: IntoNode,
//     KF: Fn(&T) -> K + 'static,
//     K: Eq + Hash + 'static,
//     T: 'static,
// {
//     fn into_node(self, cx: leptos_reactive::Scope) -> crate::Node {
//         let Self {
//             items_fn,
//             each_fn,
//             key_fn,
//         } = self;

//         let component = Component::new("Each");

//         let children = component.children.clone();

//         create_effect(cx, move |prev_hash_run| {
//             let items = items_fn();

//             let items = items.into_iter().collect::<Vec<_>>();

//             let hashed_items = items
//                 .iter()
//                 .enumerate()
//                 .map(|(idx, i)| (key_fn(&i), idx))
//                 .collect::<HashMap<_, _>>();

//             if let Some(prev_hash_run) = prev_hash_run {
//                 todo!();
//             } else {
//                 let mut children_borrow = children.borrow_mut();

//                 *children_borrow = Vec::with_capacity(items.len());

//                 for item in items {
//                     let child = each_fn(item).into_node(cx);

//                 }
//             }

//             HashRun(hashed_items)
//         });

//         todo!()
//     }
// }

// #[derive(educe::Educe)]
// #[educe(Debug)]
// struct HashRun<K, T>(#[educe(Debug(ignore))] HashMap<K, T>);

// /// Calculates the operations need to get from `a` to `b`.
// fn diff<T>(a: &[T], b: &[T]) -> Vec<DiffOp>
// where
//     T: Eq,
// {
//     todo!()
// }

// enum DiffOp {
//     Move { from: usize, to: usize },
//     Swap { between: usize },
//     Add { at: usize },
//     Remove { at: usize },
// }
