use crate::{
    effect::RenderEffect,
    signal::ArcRwSignal,
    traits::{Track, Update},
};
use or_poisoned::OrPoisoned;
use rustc_hash::FxHashMap;
use std::{
    hash::Hash,
    sync::{Arc, RwLock},
};

/// A conditional signal that only notifies subscribers when a change
/// in the source signal’s value changes whether the given function is true.
///
/// **You probably don’t need this,** but it can be a very useful optimization
/// in certain situations (e.g., “set the class `selected` if `selected() == this_row_index`)
/// because it reduces them from `O(n)` to `O(1)`.
///
/// ```
/// # use reactive_graph::computed::*;
/// # use reactive_graph::signal::*; let owner = reactive_graph::owner::Owner::new(); owner.set();
/// # use reactive_graph::prelude::*;
/// # use reactive_graph::effect::Effect;
/// # use reactive_graph::owner::StoredValue; let owner = reactive_graph::owner::Owner::new(); owner.set();
/// # tokio_test::block_on(async move {
/// # tokio::task::LocalSet::new().run_until(async move {
/// # any_spawner::Executor::init_tokio(); let owner = reactive_graph::owner::Owner::new(); owner.set();
/// # let _guard = reactive_graph::diagnostics::SpecialNonReactiveZone::enter();
/// let a = RwSignal::new(0);
/// let is_selected = Selector::new(move || a.get());
/// let total_notifications = StoredValue::new(0);
/// Effect::new_isomorphic({
///     let is_selected = is_selected.clone();
///     move |_| {
///         if is_selected.selected(5) {
///             total_notifications.update_value(|n| *n += 1);
///         }
///     }
/// });
///
/// assert_eq!(is_selected.selected(5), false);
/// assert_eq!(total_notifications.get_value(), 0);
/// a.set(5);
/// # any_spawner::Executor::tick().await;
///
/// assert_eq!(is_selected.selected(5), true);
/// assert_eq!(total_notifications.get_value(), 1);
/// a.set(5);
/// # any_spawner::Executor::tick().await;
///
/// assert_eq!(is_selected.selected(5), true);
/// assert_eq!(total_notifications.get_value(), 1);
/// a.set(4);
///
/// # any_spawner::Executor::tick().await;
/// assert_eq!(is_selected.selected(5), false);
/// # }).await;
/// # });
/// ```
#[derive(Clone)]
pub struct Selector<T>
where
    T: PartialEq + Eq + Clone + Hash + 'static,
{
    subs: Arc<RwLock<FxHashMap<T, ArcRwSignal<bool>>>>,
    v: Arc<RwLock<Option<T>>>,
    #[allow(clippy::type_complexity)]
    f: Arc<dyn Fn(&T, &T) -> bool + Send + Sync>,
    // owning the effect keeps it alive, to keep updating the selector
    #[allow(dead_code)]
    effect: Arc<RenderEffect<T>>,
}

impl<T> Selector<T>
where
    T: PartialEq + Send + Sync + Eq + Clone + Hash + 'static,
{
    /// Creates a new selector that compares values using [`PartialEq`].
    pub fn new(source: impl Fn() -> T + Send + Sync + Clone + 'static) -> Self {
        Self::new_with_fn(source, PartialEq::eq)
    }

    /// Creates a new selector that compares values by returning `true` from a comparator function
    /// if the values are the same.
    pub fn new_with_fn(
        source: impl Fn() -> T + Clone + Send + Sync + 'static,
        f: impl Fn(&T, &T) -> bool + Send + Sync + Clone + 'static,
    ) -> Self {
        let subs: Arc<RwLock<FxHashMap<T, ArcRwSignal<bool>>>> =
            Default::default();
        let v: Arc<RwLock<Option<T>>> = Default::default();
        let f = Arc::new(f) as Arc<dyn Fn(&T, &T) -> bool + Send + Sync>;

        let effect = Arc::new(RenderEffect::new_isomorphic({
            let subs = Arc::clone(&subs);
            let f = Arc::clone(&f);
            let v = Arc::clone(&v);
            move |prev: Option<T>| {
                let next_value = source();
                *v.write().or_poisoned() = Some(next_value.clone());
                if prev.as_ref() != Some(&next_value) {
                    for (key, signal) in &*subs.read().or_poisoned() {
                        if f(key, &next_value)
                            || (prev.is_some()
                                && f(key, prev.as_ref().unwrap()))
                        {
                            signal.update(|n| *n = true);
                        }
                    }
                }
                next_value
            }
        }));

        Selector { subs, v, f, effect }
    }

    /// Reactively checks whether the given key is selected.
    pub fn selected(&self, key: T) -> bool {
        let read = {
            let mut subs = self.subs.write().or_poisoned();
            subs.entry(key.clone())
                .or_insert_with(|| ArcRwSignal::new(false))
                .clone()
        };
        read.track();
        (self.f)(&key, self.v.read().or_poisoned().as_ref().unwrap())
    }

    /// Removes the listener for the given key.
    pub fn remove(&self, key: &T) {
        let mut subs = self.subs.write().or_poisoned();
        subs.remove(key);
    }

    /// Clears the listeners for all keys.
    pub fn clear(&self) {
        let mut subs = self.subs.write().or_poisoned();
        subs.clear();
    }
}
