use crate::{Runtime, Scope, ScopeId, Source, Subscriber};
use serde::{Deserialize, Serialize};
use std::{any::Any, cell::RefCell, collections::HashSet, fmt::Debug, marker::PhantomData};

/// Creates a signal, the basic reactive primitive.
///
/// A signal is a piece of data that may change over time,
/// and notifies other code when it has changed. This is the
/// core primitive of Leptos’s reactive system.
///
/// Takes a reactive [Scope] and the initial value as arguments,
/// and returns a tuple containing a [ReadSignal] and a [WriteSignal],
/// each of which can be called as a function.
///
/// ```
/// # use leptos_reactive::*;
/// # create_scope(|cx| {
/// let (count, set_count) = create_signal(cx, 0);
///
/// // ✅ calling the getter clones and returns the value
/// assert_eq!(count(), 0);
///
/// // ✅ calling the setter sets the value
/// set_count(1);
/// assert_eq!(count(), 1);
///
/// // ❌ don't try to call the getter within the setter
/// // set_count(count() + 1);
///
/// // ✅ instead, use .update() to mutate the value in place
/// set_count.update(|count: &mut i32| *count += 1);
/// assert_eq!(count(), 2);
///
/// // ✅ you can create "derived signals" with the same Fn() -> T interface
/// let double_count = move || count() * 2; // signals are `Copy` so you can `move` them anywhere
/// set_count(0);
/// assert_eq!(double_count(), 0);
/// set_count(1);
/// assert_eq!(double_count(), 2);
/// # }).dispose();
/// #
/// ```
pub fn create_signal<T>(cx: Scope, value: T) -> (ReadSignal<T>, WriteSignal<T>)
where
    T: Clone + Debug,
{
    let state = SignalState::new(value);
    let id = cx.push_signal(state);

    let read = ReadSignal {
        runtime: cx.runtime,
        scope: cx.id,
        id,
        ty: PhantomData,
    };

    let write = WriteSignal {
        runtime: cx.runtime,
        scope: cx.id,
        id,
        ty: PhantomData,
    };

    (read, write)
}

/// The getter for a reactive signal.
///
/// A signal is a piece of data that may change over time,
/// and notifies other code when it has changed. This is the
/// core primitive of Leptos’s reactive system.
///
/// Calling [ReadSignal::get] within an effect will cause that effect
/// to subscribe to the signal, and to re-run whenever the value of
/// the signal changes.
///
/// `ReadSignal` implements [Fn], so that `value()` and `value.get()` are identical.
///
/// `ReadSignal` is also [Copy] and `'static`, so it can very easily moved into closures
/// or copied structs.
///
/// ```
/// # use leptos_reactive::*;
/// # create_scope(|cx| {
/// let (count, set_count) = create_signal(cx, 0);
///
/// // ✅ calling the getter clones and returns the value
/// assert_eq!(count(), 0);
///
/// // ✅ calling the setter sets the value
/// set_count(1);
/// assert_eq!(count(), 1);
///
/// // ❌ don't try to call the getter within the setter
/// // set_count(count() + 1);
///
/// // ✅ instead, use .update() to mutate the value in place
/// set_count.update(|count: &mut i32| *count += 1);
/// assert_eq!(count(), 2);
///
/// // ✅ you can create "derived signals" with the same Fn() -> T interface
/// let double_count = move || count() * 2; // signals are `Copy` so you can `move` them anywhere
/// set_count(0);
/// assert_eq!(double_count(), 0);
/// set_count(1);
/// assert_eq!(double_count(), 2);
/// # }).dispose();
/// #
/// ```
#[derive(Debug, PartialEq, Eq, Hash)]
pub struct ReadSignal<T>
where
    T: 'static,
{
    runtime: &'static Runtime,
    pub(crate) scope: ScopeId,
    pub(crate) id: SignalId,
    pub(crate) ty: PhantomData<T>,
}

impl<T> ReadSignal<T>
where
    T: Debug,
{
    /// Clones and returns the current value of the signal, and subscribes
    /// the running effect to this signal.
    ///
    /// If you want to get the value without cloning it, use [ReadSignal::with].
    /// (`value.get()` is equivalent to `value.with(T::clone)`.)
    /// ```
    /// # use leptos_reactive::*;
    /// # create_scope(|cx| {
    /// let (count, set_count) = create_signal(cx, 0);
    ///
    /// // calling the getter clones and returns the value
    /// assert_eq!(count(), 0);
    /// });
    /// ```
    pub fn get(&self) -> T
    where
        T: Clone,
    {
        self.with(T::clone)
    }

    /// Applies a function to the current value of the signal, and subscribes
    /// the running effect to this signal.
    ///
    /// If you want to get the value without cloning it, use [ReadSignal::with].
    /// ```
    /// # use leptos_reactive::*;
    /// # create_scope(|cx| {
    /// let (name, set_name) = create_signal(cx, "Alice".to_string());
    ///
    /// // ❌ unnecessarily clones the string
    /// let first_char = move || name().chars().next().unwrap();
    /// assert_eq!(first_char(), 'A');
    ///
    /// // ✅ gets the first char without cloning the `String`
    /// let first_char = move || name.with(|n| n.chars().next().unwrap());
    /// assert_eq!(first_char(), 'A');
    /// set_name("Bob".to_string());
    /// assert_eq!(first_char(), 'B');
    /// });
    /// ```
    pub fn with<U>(&self, f: impl Fn(&T) -> U) -> U {
        if let Some(running_subscriber) = self.runtime.running_effect() {
            self.runtime
                .any_effect(running_subscriber.0, |running_effect| {
                    self.add_subscriber(Subscriber(running_subscriber.0));
                    running_effect.subscribe_to(Source((self.scope, self.id)));
                });
        }

        self.runtime
            .signal((self.scope, self.id), |signal_state: &SignalState<T>| {
                (f)(&signal_state.value.borrow())
            })
    }

    fn add_subscriber(&self, subscriber: Subscriber) {
        self.runtime
            .signal((self.scope, self.id), |signal_state: &SignalState<T>| {
                signal_state.subscribers.borrow_mut().insert(subscriber);
            })
    }
}

impl<T> Clone for ReadSignal<T> {
    fn clone(&self) -> Self {
        Self {
            runtime: self.runtime,
            scope: self.scope,
            id: self.id,
            ty: PhantomData,
        }
    }
}

impl<T> Copy for ReadSignal<T> {}

impl<T> FnOnce<()> for ReadSignal<T>
where
    T: Debug + Clone,
{
    type Output = T;

    extern "rust-call" fn call_once(self, _args: ()) -> Self::Output {
        self.get()
    }
}

impl<T> FnMut<()> for ReadSignal<T>
where
    T: Debug + Clone,
{
    extern "rust-call" fn call_mut(&mut self, _args: ()) -> Self::Output {
        self.get()
    }
}

impl<T> Fn<()> for ReadSignal<T>
where
    T: Debug + Clone,
{
    extern "rust-call" fn call(&self, _args: ()) -> Self::Output {
        self.get()
    }
}

/// The setter for a reactive signal.
///
/// A signal is a piece of data that may change over time,
/// and notifies other code when it has changed. This is the
/// core primitive of Leptos’s reactive system.
///
/// Calling [WriteSignal::update] will mutate the signal’s value in place,
/// and notify all subscribers that the signal’s value has changed.
///
/// `ReadSignal` implements [Fn], such that `set_value(new_value)` is equivalent to
/// `set_value.update(|value| *value = new_value)`.
///
/// `WriteSignal` is [Copy] and `'static`, so it can very easily moved into closures
/// or copied structs.
///
/// ```
/// # use leptos_reactive::*;
/// # create_scope(|cx| {
/// let (count, set_count) = create_signal(cx, 0);
///
/// // ✅ calling the setter sets the value
/// set_count(1);
/// assert_eq!(count(), 1);
///
/// // ❌ don't try to call the getter within the setter
/// // set_count(count() + 1);
///
/// // ✅ instead, use .update() to mutate the value in place
/// set_count.update(|count: &mut i32| *count += 1);
/// assert_eq!(count(), 2);
/// # }).dispose();
/// #
/// ```
#[derive(Debug, PartialEq, Eq, Hash)]
pub struct WriteSignal<T>
where
    T: Clone + 'static,
{
    runtime: &'static Runtime,
    pub(crate) scope: ScopeId,
    pub(crate) id: SignalId,
    pub(crate) ty: PhantomData<T>,
}

impl<T> WriteSignal<T>
where
    T: Clone + 'static,
{
    /// Applies a function to the current value and notifies subscribers
    /// that the signal has changed.
    ///
    /// **Note:** `update()` does not auto-memoize, i.e., it will notify subscribers
    /// even if the value has not actually changed.
    /// ```
    /// # use leptos_reactive::*;
    /// # create_scope(|cx| {
    /// let (count, set_count) = create_signal(cx, 0);
    ///
    /// // notifies subscribers
    /// set_count.update(|n| *n = 1); // it's easier just to call set_count(1), though!
    /// assert_eq!(count(), 1);
    ///
    /// // you can include arbitrary logic in this update function
    /// // also notifies subscribers, even though the value hasn't changed
    /// set_count.update(|n| if *n > 3 { *n += 1 });
    /// assert_eq!(count(), 1);
    /// # }).dispose();
    /// ```
    pub fn update(&self, f: impl FnOnce(&mut T)) {
        self.runtime
            .signal((self.scope, self.id), |signal_state: &SignalState<T>| {
                // update value
                (f)(&mut *signal_state.value.borrow_mut());

                // notify subscribers
                // if any of them are in scopes that have been disposed, unsubscribe
                let subs = { signal_state.subscribers.borrow().clone() };
                let mut dropped_subs = Vec::new();
                for subscriber in subs.iter() {
                    if subscriber.try_run(self.runtime).is_err() {
                        dropped_subs.push(subscriber);
                    }
                }
                for sub in dropped_subs {
                    signal_state.subscribers.borrow_mut().remove(sub);
                }
            })
    }
}

impl<T> Clone for WriteSignal<T>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        Self {
            runtime: self.runtime,
            scope: self.scope,
            id: self.id,
            ty: PhantomData,
        }
    }
}

impl<T> Copy for WriteSignal<T> where T: Clone {}

impl<T> FnOnce<(T,)> for WriteSignal<T>
where
    T: Clone + 'static,
{
    type Output = ();

    extern "rust-call" fn call_once(self, args: (T,)) -> Self::Output {
        self.update(move |n| *n = args.0)
    }
}

impl<T> FnMut<(T,)> for WriteSignal<T>
where
    T: Clone + 'static,
{
    extern "rust-call" fn call_mut(&mut self, args: (T,)) -> Self::Output {
        self.update(move |n| *n = args.0)
    }
}

impl<T> Fn<(T,)> for WriteSignal<T>
where
    T: Clone + 'static,
{
    extern "rust-call" fn call(&self, args: (T,)) -> Self::Output {
        self.update(move |n| *n = args.0)
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct SignalId(pub(crate) usize);

//#[derive(Debug)]
pub(crate) struct SignalState<T> {
    value: RefCell<T>,
    subscribers: RefCell<HashSet<Subscriber>>,
}

impl<T> Debug for SignalState<T>
where
    T: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SignalState")
            .field("value", &*self.value.borrow())
            .field("subscribers", &*self.subscribers.borrow())
            .finish()
    }
}

impl<T> SignalState<T>
where
    T: Debug,
{
    pub fn new(value: T) -> Self {
        Self {
            value: RefCell::new(value),
            subscribers: Default::default(),
        }
    }
}

pub(crate) trait AnySignal: Debug {
    fn unsubscribe(&self, subscriber: Subscriber);

    fn as_any(&self) -> &dyn Any;
}

impl<T> AnySignal for SignalState<T>
where
    T: Debug + 'static,
{
    fn unsubscribe(&self, subscriber: Subscriber) {
        self.subscribers.borrow_mut().remove(&subscriber);
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
