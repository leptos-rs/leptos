use leptos_reactive::{BoundedScope, ReadSignal, Scope, ScopeDisposer};
use std::{collections::HashMap, hash::Hash};

/// Function that maps a `Vec` to another `Vec` via a map function. The mapped `Vec` is lazy
/// computed; its value will only be updated when requested. Modifications to the
/// input `Vec` are diffed using keys to prevent recomputing values that have not changed.
///
/// This function is the underlying utility behind `Keyed`.
///
/// # Params
/// * `list` - The list to be mapped. The list must be a [`ReadSignal`] (obtained from a [`Signal`])
///   and therefore reactive.
/// * `map_fn` - A closure that maps from the input type to the output type.
/// * `key_fn` - A closure that returns an _unique_ key to each entry.
///
///  _Credits: Based on implementation for [Sycamore](https://github.com/sycamore-rs/sycamore/blob/53735aab9ef72b98439b4d2eaeb85a97f7f32775/packages/sycamore-reactive/src/iter.rs),
/// which is in turned based on on the TypeScript implementation in <https://github.com/solidjs/solid>_
pub fn map_keyed<'a, T, U, K>(
    cx: Scope<'a>,
    list: &'a ReadSignal<Vec<T>>,
    map_fn: impl for<'child_lifetime> Fn(BoundedScope<'child_lifetime, 'a>, T) -> U + 'a,
    key_fn: impl Fn(&T) -> K + 'a,
) -> &'a ReadSignal<Vec<U>>
where
    T: PartialEq + Clone + 'a,
    K: Eq + Hash,
    U: Clone,
{
    // Previous state used for diffing.
    let mut items = Vec::new();
    let mut mapped: Vec<U> = Vec::new();
    let mut disposers: Vec<Option<ScopeDisposer<'a>>> = Vec::new();

    let (item_signal, set_item_signal) = cx.signal(Vec::new());

    // Diff and update signal each time list is updated.
    cx.create_effect(move || {
        let new_items = list.get();
        if new_items.is_empty() {
            // Fast path for removing all items.
            for disposer in std::mem::take(&mut disposers) {
                unsafe {
                    disposer.unwrap().dispose();
                }
            }
            mapped = Vec::new();
        } else if items.is_empty() {
            // Fast path for creating items when the existing list is empty.
            for new_item in new_items.iter() {
                let mut value = None;
                let new_disposer = cx.child_scope(|cx| {
                    // SAFETY: f takes the same parameter as the argument to create_child_scope.
                    value = Some(map_fn(
                        unsafe { std::mem::transmute(cx) },
                        (*new_item).clone(),
                    ));
                });
                mapped.push(value.unwrap());
                disposers.push(Some(new_disposer));
            }
        } else {
            let mut temp = vec![None; new_items.len()];
            let mut temp_disposers: Vec<Option<ScopeDisposer<'a>>> =
                (0..new_items.len()).map(|_| None).collect();

            // Skip common prefix.
            let min_len = usize::min(items.len(), new_items.len());
            let start = items
                .iter()
                .zip(new_items.iter())
                .position(|(a, b)| a != b)
                .unwrap_or(min_len);

            // Skip common suffix.
            let mut end = items.len();
            let mut new_end = new_items.len();
            #[allow(clippy::suspicious_operation_groupings)]
            // FIXME: make code clearer so that clippy won't complain
            while end > start && new_end > start && items[end - 1] == new_items[new_end - 1] {
                end -= 1;
                new_end -= 1;
                temp[new_end] = Some(mapped[end].clone());
                temp_disposers[new_end] = disposers[end].take();
            }

            // 0) Prepare a map of indices in newItems. Scan backwards so we encounter them in
            // natural order.
            let mut new_indices = HashMap::with_capacity(new_end - start);

            // Indexes for new_indices_next are shifted by start because values at 0..start are
            // always None.
            let mut new_indices_next = vec![None; new_end - start];
            for j in (start..new_end).rev() {
                let item = &new_items[j];
                let i = new_indices.get(&key_fn(item));
                new_indices_next[j - start] = i.copied();
                new_indices.insert(key_fn(item), j);
            }

            // 1) Step through old items and see if they can be found in new set; if so, mark
            // them as moved.
            for i in start..end {
                let item = &items[i];
                if let Some(j) = new_indices.get(&key_fn(item)).copied() {
                    // Moved. j is index of item in new_items.
                    temp[j] = Some(mapped[i].clone());
                    temp_disposers[j] = disposers[i].take();
                    new_indices_next[j - start].and_then(|j| new_indices.insert(key_fn(item), j));
                } else {
                    // Create new.
                    unsafe {
                        disposers[i].take().unwrap().dispose();
                    }
                }
            }

            // 2) Set all the new values, pulling from the moved array if copied, otherwise
            // entering the new value.
            for j in start..new_items.len() {
                if matches!(temp.get(j), Some(Some(_))) {
                    // Pull from moved array.
                    if j >= mapped.len() {
                        debug_assert_eq!(mapped.len(), j);
                        mapped.push(temp[j].clone().unwrap());
                        disposers.push(temp_disposers[j].take());
                    } else {
                        mapped[j] = temp[j].clone().unwrap();
                        disposers[j] = temp_disposers[j].take();
                    }
                } else {
                    // Create new value.
                    let mut tmp = None;
                    let new_item = new_items[j].clone();
                    let new_disposer = cx.child_scope(|cx| {
                        // SAFETY: f takes the same parameter as the argument to create_child_scope.
                        tmp = Some(map_fn(unsafe { std::mem::transmute(cx) }, new_item.clone()));
                    });
                    if mapped.len() > j {
                        mapped[j] = tmp.unwrap();
                        disposers[j] = Some(new_disposer);
                    } else {
                        mapped.push(tmp.unwrap());
                        disposers.push(Some(new_disposer));
                    }
                }
            }
        }

        // 3) In case the new set is shorter than the old, set the length of the mapped array.
        mapped.truncate(new_items.len());
        disposers.truncate(new_items.len());

        // 4) Save a copy of the mapped items for the next update.
        items = new_items.to_vec();

        // 5) Update signal to trigger updates.
        set_item_signal(|n| *n = mapped.clone());
    });

    item_signal
}
