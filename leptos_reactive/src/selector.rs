#![forbid(unsafe_code)]
use crate::{
    create_isomorphic_effect, create_signal, ReadSignal, Scope, SignalUpdate,
    WriteSignal,
};
use std::{cell::RefCell, collections::HashMap, hash::Hash, rc::Rc};

/// Creates a conditional signal that only notifies subscribers when a change
/// in the source signal’s value changes whether it is equal to the key value
/// (as determined by [`PartialEq`].)
///
/// **You probably don’t need this,** but it can be a very useful optimization
/// in certain situations (e.g., “set the class `selected` if `selected() == this_row_index`)
/// because it reduces them from `O(n)` to `O(1)`.
///
/// ```
/// # use leptos_reactive::*;
/// # use std::rc::Rc;
/// # use std::cell::RefCell;
/// # create_scope(create_runtime(), |cx| {
/// let (a, set_a) = create_signal(cx, 0);
/// let is_selected = create_selector(cx, move || a.get());
/// let total_notifications = Rc::new(RefCell::new(0));
/// let not = Rc::clone(&total_notifications);
/// create_isomorphic_effect(cx, {
///     let is_selected = is_selected.clone();
///     move |_| {
///         if is_selected(5) {
///             *not.borrow_mut() += 1;
///         }
///     }
/// });
///
/// assert_eq!(is_selected(5), false);
/// assert_eq!(*total_notifications.borrow(), 0);
/// set_a.set(5);
/// assert_eq!(is_selected(5), true);
/// assert_eq!(*total_notifications.borrow(), 1);
/// set_a.set(5);
/// assert_eq!(is_selected(5), true);
/// assert_eq!(*total_notifications.borrow(), 1);
/// set_a.set(4);
/// assert_eq!(is_selected(5), false);
///  # })
///  # .dispose()
/// ```
#[inline(always)]
pub fn create_selector<T>(
    cx: Scope,
    source: impl Fn() -> T + Clone + 'static,
) -> impl Fn(T) -> bool + Clone
where
    T: PartialEq + Eq + Clone + Hash + 'static,
{
    create_selector_with_fn(cx, source, PartialEq::eq)
}

/// Creates a conditional signal that only notifies subscribers when a change
/// in the source signal’s value changes whether the given function is true.
///
/// **You probably don’t need this,** but it can be a very useful optimization
/// in certain situations (e.g., “set the class `selected` if `selected() == this_row_index`)
/// because it reduces them from `O(n)` to `O(1)`.
pub fn create_selector_with_fn<T>(
    cx: Scope,
    source: impl Fn() -> T + Clone + 'static,
    f: impl Fn(&T, &T) -> bool + Clone + 'static,
) -> impl Fn(T) -> bool + Clone
where
    T: PartialEq + Eq + Clone + Hash + 'static,
{
    #[allow(clippy::type_complexity)]
    let subs: Rc<
        RefCell<HashMap<T, (ReadSignal<bool>, WriteSignal<bool>)>>,
    > = Rc::new(RefCell::new(HashMap::new()));
    let v = Rc::new(RefCell::new(None));

    create_isomorphic_effect(cx, {
        let subs = Rc::clone(&subs);
        let f = f.clone();
        let v = Rc::clone(&v);
        move |prev: Option<T>| {
            let next_value = source();
            *v.borrow_mut() = Some(next_value.clone());
            if prev.as_ref() != Some(&next_value) {
                let subs = { subs.borrow().clone() };
                for (key, signal) in subs.into_iter() {
                    if f(&key, &next_value)
                        || (prev.is_some() && f(&key, prev.as_ref().unwrap()))
                    {
                        signal.1.update(|n| *n = true);
                    }
                }
            }
            next_value
        }
    });

    move |key| {
        let mut subs = subs.borrow_mut();
        let (read, _) = subs
            .entry(key.clone())
            .or_insert_with(|| create_signal(cx, false));
        _ = read.try_with(|n| *n);
        f(&key, v.borrow().as_ref().unwrap())
    }
}
