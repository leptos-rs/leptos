use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    fmt::Debug,
    hash::Hash,
    rc::Rc,
};

use crate::{create_effect, create_signal, ReadSignal, Scope, WriteSignal};

/// Creates a conditional signal that only notifies subscribers when a change
/// in the source matches their key.
/// ```
/// # use leptos_reactive::{create_effect, create_scope, create_selector, create_signal};
/// # use std::rc::Rc;
/// # use std::cell::RefCell;
/// # create_scope(|cx| {
///    let (a, set_a) = create_signal(cx, 0);
///    let is_selected = create_selector(cx, a);
///    let total_notifications = Rc::new(RefCell::new(0));
///    let not = Rc::clone(&total_notifications);
///    create_effect(cx, {let is_selected = is_selected.clone(); move |_| {
///      if is_selected(5) {
///        *not.borrow_mut() += 1;
///      }
///    }});
///
///    assert_eq!(is_selected(5), false);
///    assert_eq!(*total_notifications.borrow(), 0);
///    set_a(5);
///    assert_eq!(is_selected(5), true);
///    assert_eq!(*total_notifications.borrow(), 1);
///    set_a(5);
///    assert_eq!(is_selected(5), true);
///    assert_eq!(*total_notifications.borrow(), 1);
///    set_a(4);
///    assert_eq!(is_selected(5), false);
///    //assert_eq!(*total_notifications.borrow(), 2);
///  # })
///  # .dispose()
/// ```
pub fn create_selector<T>(
    cx: Scope,
    source: impl Fn() -> T + Clone + 'static,
) -> impl Fn(T) -> bool + Clone
where
    T: PartialEq + Eq + Debug + Clone + Hash + 'static,
{
    create_selector_with_fn(cx, source, |a, b| a == b)
}

pub fn create_selector_with_fn<T>(
    cx: Scope,
    source: impl Fn() -> T + Clone + 'static,
    f: impl Fn(&T, &T) -> bool + Clone + 'static,
) -> impl Fn(T) -> bool + Clone
where
    T: PartialEq + Eq + Debug + Clone + Hash + 'static,
{
    let subs: Rc<RefCell<HashMap<T, (ReadSignal<bool>, WriteSignal<bool>)>>> =
        Rc::new(RefCell::new(HashMap::new()));
    let v = Rc::new(RefCell::new(None));

    create_effect(cx, {
        let subs = Rc::clone(&subs);
        let f = f.clone();
        let v = Rc::clone(&v);
        move |prev: Option<T>| {
            let next_value = source();
            *v.borrow_mut() = Some(next_value.clone());
            if prev.as_ref() != Some(&next_value) {
                let subs = { subs.borrow().clone() };
                for (key, signal) in subs.into_iter() {
                    if f(&key, &next_value) || (prev.is_some() && f(&key, prev.as_ref().unwrap())) {
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
        _ = read();
        f(&key, v.borrow().as_ref().unwrap())
    }
}
