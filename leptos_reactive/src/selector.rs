use crate::{
    create_isomorphic_effect, create_rw_signal, runtime::with_owner, Owner,
    RwSignal, SignalUpdate, SignalWith,
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
/// # let runtime = create_runtime();
/// let (a, set_a) = create_signal(0);
/// let is_selected = create_selector(move || a.get());
/// let total_notifications = Rc::new(RefCell::new(0));
/// let not = Rc::clone(&total_notifications);
/// create_isomorphic_effect({
///     let is_selected = is_selected.clone();
///     move |_| {
///         if is_selected.selected(5) {
///             *not.borrow_mut() += 1;
///         }
///     }
/// });
///
/// assert_eq!(is_selected.selected(5), false);
/// assert_eq!(*total_notifications.borrow(), 0);
/// set_a.set(5);
/// assert_eq!(is_selected.selected(5), true);
/// assert_eq!(*total_notifications.borrow(), 1);
/// set_a.set(5);
/// assert_eq!(is_selected.selected(5), true);
/// assert_eq!(*total_notifications.borrow(), 1);
/// set_a.set(4);
/// assert_eq!(is_selected.selected(5), false);
///  # runtime.dispose()
/// ```
#[inline(always)]
pub fn create_selector<T>(
    source: impl Fn() -> T + Clone + 'static,
) -> Selector<T>
where
    T: PartialEq + Eq + Clone + Hash + 'static,
{
    create_selector_with_fn(source, PartialEq::eq)
}

/// Creates a conditional signal that only notifies subscribers when a change
/// in the source signal’s value changes whether the given function is true.
///
/// **You probably don’t need this,** but it can be a very useful optimization
/// in certain situations (e.g., “set the class `selected` if `selected() == this_row_index`)
/// because it reduces them from `O(n)` to `O(1)`.
pub fn create_selector_with_fn<T>(
    source: impl Fn() -> T + 'static,
    f: impl Fn(&T, &T) -> bool + Clone + 'static,
) -> Selector<T>
where
    T: PartialEq + Eq + Clone + Hash + 'static,
{
    #[allow(clippy::type_complexity)]
    let subs: Rc<RefCell<HashMap<T, RwSignal<bool>>>> =
        Rc::new(RefCell::new(HashMap::new()));
    let v = Rc::new(RefCell::new(None));
    let owner = Owner::current()
        .expect("create_selector called outside the reactive system");
    let f = Rc::new(f) as Rc<dyn Fn(&T, &T) -> bool>;

    create_isomorphic_effect({
        let subs = Rc::clone(&subs);
        let f = Rc::clone(&f);
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
                        signal.update(|n| *n = true);
                    }
                }
            }
            next_value
        }
    });

    Selector { subs, v, owner, f }
}

/// A conditional signal that only notifies subscribers when a change
/// in the source signal’s value changes whether the given function is true.
#[derive(Clone)]
pub struct Selector<T>
where
    T: PartialEq + Eq + Clone + Hash + 'static,
{
    subs: Rc<RefCell<HashMap<T, RwSignal<bool>>>>,
    v: Rc<RefCell<Option<T>>>,
    owner: Owner,
    #[allow(clippy::type_complexity)] // lol
    f: Rc<dyn Fn(&T, &T) -> bool>,
}

impl<T> core::fmt::Debug for Selector<T>
where
    T: PartialEq + Eq + Clone + Hash + 'static,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Selector").finish()
    }
}

impl<T> Selector<T>
where
    T: PartialEq + Eq + Clone + Hash + 'static,
{
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
    /// # let runtime = create_runtime();
    /// let a = RwSignal::new(0);
    /// let is_selected = Selector::new(move || a.get());
    /// let total_notifications = Rc::new(RefCell::new(0));
    /// let not = Rc::clone(&total_notifications);
    /// create_isomorphic_effect({
    ///     let is_selected = is_selected.clone();
    ///     move |_| {
    ///         if is_selected.selected(5) {
    ///             *not.borrow_mut() += 1;
    ///         }
    ///     }
    /// });
    ///
    /// assert_eq!(is_selected.selected(5), false);
    /// assert_eq!(*total_notifications.borrow(), 0);
    /// a.set(5);
    /// assert_eq!(is_selected.selected(5), true);
    /// assert_eq!(*total_notifications.borrow(), 1);
    /// a.set(5);
    /// assert_eq!(is_selected.selected(5), true);
    /// assert_eq!(*total_notifications.borrow(), 1);
    /// a.set(4);
    /// assert_eq!(is_selected.selected(5), false);
    ///  # runtime.dispose()
    /// ```
    #[inline(always)]
    #[track_caller]
    pub fn new(source: impl Fn() -> T + Clone + 'static) -> Self {
        create_selector_with_fn(source, PartialEq::eq)
    }

    /// Reactively checks whether the given key is selected.
    pub fn selected(&self, key: T) -> bool {
        let owner = self.owner;
        let read = {
            let mut subs = self.subs.borrow_mut();
            *(subs.entry(key.clone()).or_insert_with(|| {
                with_owner(owner, || create_rw_signal(false))
            }))
        };
        _ = read.try_with(|n| *n);
        (self.f)(&key, self.v.borrow().as_ref().unwrap())
    }

    /// Removes the listener for the given key.
    pub fn remove(&self, key: &T) {
        let mut subs = self.subs.borrow_mut();
        subs.remove(key);
    }
}
