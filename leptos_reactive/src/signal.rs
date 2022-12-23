use crate::{
    debug_warn,
    runtime::{with_runtime, RuntimeId},
    spawn_local, Runtime, Scope, ScopeProperty, UntrackedGettableSignal, UntrackedSettableSignal,
};
use futures::Stream;
use std::{fmt::Debug, marker::PhantomData};
use thiserror::Error;

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
/// # create_scope(create_runtime(), |cx| {
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
pub fn create_signal<T>(cx: Scope, value: T) -> (ReadSignal<T>, WriteSignal<T>) {
    let s = cx.runtime.create_signal(value);
    cx.with_scope_property(|prop| prop.push(ScopeProperty::Signal(s.0.id)));
    s
}

/// Creates a signal that always contains the most recent value emitted by a [Stream].
/// If the stream has not yet emitted a value since the signal was created, the signal's
/// value will be `None`.
pub fn create_signal_from_stream<T>(
    cx: Scope,
    mut stream: impl Stream<Item = T> + Unpin + 'static,
) -> ReadSignal<Option<T>> {
    use futures::StreamExt;

    let (read, write) = create_signal(cx, None);
    spawn_local(async move {
        while let Some(value) = stream.next().await {
            write.set(Some(value));
        }
    });
    read
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
/// # create_scope(create_runtime(), |cx| {
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
    pub(crate) runtime: RuntimeId,
    pub(crate) id: SignalId,
    pub(crate) ty: PhantomData<T>,
}

impl<T> UntrackedGettableSignal<T> for ReadSignal<T> {
    fn get_untracked(&self) -> T
    where
        T: Clone,
    {
        self.with_no_subscription(|v| v.clone())
    }

    fn with_untracked<O>(&self, f: impl FnOnce(&T) -> O) -> O {
        self.with_no_subscription(f)
    }
}

impl<T> ReadSignal<T>
where
    T: 'static,
{
    /// Applies a function to the current value of the signal, and subscribes
    /// the running effect to this signal.
    /// ```
    /// # use leptos_reactive::*;
    /// # create_scope(create_runtime(), |cx| {
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
    pub fn with<U>(&self, f: impl FnOnce(&T) -> U) -> U {
        self.id.with(self.runtime, f)
    }

    pub(crate) fn with_no_subscription<U>(&self, f: impl FnOnce(&T) -> U) -> U {
        self.id.with_no_subscription(self.runtime, f)
    }

    #[cfg(feature = "hydrate")]
    pub(crate) fn subscribe(&self) {
        with_runtime(self.runtime, |runtime| self.id.subscribe(runtime))
    }

    /// Clones and returns the current value of the signal, and subscribes
    /// the running effect to this signal.
    ///
    /// If you want to get the value without cloning it, use [ReadSignal::with].
    /// (`value.get()` is equivalent to `value.with(T::clone)`.)
    /// ```
    /// # use leptos_reactive::*;
    /// # create_scope(create_runtime(), |cx| {
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
        self.id.with(self.runtime, T::clone)
    }

    /// Applies the function to the current Signal, if it exists, and subscribes
    /// the running effect.
    pub(crate) fn try_with<U>(&self, f: impl FnOnce(&T) -> U) -> Result<U, SignalError> {
        with_runtime(self.runtime, |runtime| self.id.try_with(runtime, f))
    }

    /// Generates a [Stream] that emits the new value of the signal whenever it changes.
    pub fn to_stream(&self) -> impl Stream<Item = T>
    where
        T: Clone,
    {
        let (tx, rx) = futures::channel::mpsc::unbounded();
        let id = self.id;
        let runtime = self.runtime;
        // TODO: because it's not attached to a scope, this effect will leak if the scope is disposed
        runtime.create_effect(move |_| {
            _ = tx.unbounded_send(id.with(runtime, T::clone));
        });
        rx
    }
}

impl<T> Clone for ReadSignal<T> {
    fn clone(&self) -> Self {
        Self {
            runtime: self.runtime,
            id: self.id,
            ty: PhantomData,
        }
    }
}

impl<T> Copy for ReadSignal<T> {}

#[cfg(not(feature = "stable"))]
impl<T> FnOnce<()> for ReadSignal<T>
where
    T: Clone,
{
    type Output = T;

    extern "rust-call" fn call_once(self, _args: ()) -> Self::Output {
        self.get()
    }
}

#[cfg(not(feature = "stable"))]
impl<T> FnMut<()> for ReadSignal<T>
where
    T: Clone,
{
    extern "rust-call" fn call_mut(&mut self, _args: ()) -> Self::Output {
        self.get()
    }
}

#[cfg(not(feature = "stable"))]
impl<T> Fn<()> for ReadSignal<T>
where
    T: Clone,
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
/// # create_scope(create_runtime(), |cx| {
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
    T: 'static,
{
    pub(crate) runtime: RuntimeId,
    pub(crate) id: SignalId,
    pub(crate) ty: PhantomData<T>,
}

impl<T> UntrackedSettableSignal<T> for WriteSignal<T>
where
    T: 'static,
{
    fn set_untracked(&self, new_value: T) {
        self.id
            .update_with_no_effect(self.runtime, |v| *v = new_value);
    }

    fn update_untracked(&self, f: impl FnOnce(&mut T)) {
        self.id.update_with_no_effect(self.runtime, f);
    }

    fn update_returning_untracked<U>(&self, f: impl FnOnce(&mut T) -> U) -> Option<U> {
        self.id.update_with_no_effect(self.runtime, f)
    }
}

impl<T> WriteSignal<T>
where
    T: 'static,
{
    /// Applies a function to the current value to mutate it in place
    /// and notifies subscribers that the signal has changed.
    ///
    /// **Note:** `update()` does not auto-memoize, i.e., it will notify subscribers
    /// even if the value has not actually changed.
    /// ```
    /// # use leptos_reactive::*;
    /// # create_scope(create_runtime(), |cx| {
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
        self.id.update(self.runtime, f);
    }

    /// Applies a function to the current value to mutate it in place
    /// and notifies subscribers that the signal has changed.
    /// Forwards the return value of the closure if the closure was called
    ///
    /// **Note:** `update()` does not auto-memoize, i.e., it will notify subscribers
    /// even if the value has not actually changed.
    /// ```
    /// # use leptos_reactive::*;
    /// # create_scope(create_runtime(), |cx| {
    /// let (count, set_count) = create_signal(cx, 0);
    ///
    /// // notifies subscribers
    /// let value = set_count.update_returning(|n| { *n = 1; *n * 10 });
    /// assert_eq!(value, Some(10));
    /// assert_eq!(count(), 1);
    ///
    /// let value = set_count.update_returning(|n| { *n += 1; *n * 10 });
    /// assert_eq!(value, Some(20));
    /// assert_eq!(count(), 2);
    /// # }).dispose();
    /// ```
    pub fn update_returning<U>(&self, f: impl FnOnce(&mut T) -> U) -> Option<U> {
        self.id.update(self.runtime, f)
    }

    /// Sets the signal’s value and notifies subscribers.
    ///
    /// **Note:** `set()` does not auto-memoize, i.e., it will notify subscribers
    /// even if the value has not actually changed.
    /// ```
    /// # use leptos_reactive::*;
    /// # create_scope(create_runtime(), |cx| {
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
    pub fn set(&self, new_value: T) {
        self.id.update(self.runtime, |n| *n = new_value);
    }
}

impl<T> Clone for WriteSignal<T> {
    fn clone(&self) -> Self {
        Self {
            runtime: self.runtime,
            id: self.id,
            ty: PhantomData,
        }
    }
}

impl<T> Copy for WriteSignal<T> {}

#[cfg(not(feature = "stable"))]
impl<T> FnOnce<(T,)> for WriteSignal<T>
where
    T: 'static,
{
    type Output = ();

    extern "rust-call" fn call_once(self, args: (T,)) -> Self::Output {
        self.update(move |n| *n = args.0)
    }
}

#[cfg(not(feature = "stable"))]
impl<T> FnMut<(T,)> for WriteSignal<T>
where
    T: 'static,
{
    extern "rust-call" fn call_mut(&mut self, args: (T,)) -> Self::Output {
        self.update(move |n| *n = args.0)
    }
}

#[cfg(not(feature = "stable"))]
impl<T> Fn<(T,)> for WriteSignal<T>
where
    T: 'static,
{
    extern "rust-call" fn call(&self, args: (T,)) -> Self::Output {
        self.update(move |n| *n = args.0)
    }
}

/// Creates a reactive signal with the getter and setter unified in one value.
/// You may prefer this style, or it may be easier to pass around in a context
/// or as a function argument.
/// ```
/// # use leptos_reactive::*;
/// # create_scope(create_runtime(), |cx| {
/// let count = create_rw_signal(cx, 0);
///
/// // ✅ set the value
/// count.set(1);
/// assert_eq!(count(), 1);
///
/// // ❌ don't try to call the getter within the setter
/// // count.set(count.get() + 1);
///
/// // ✅ instead, use .update() to mutate the value in place
/// count.update(|count: &mut i32| *count += 1);
/// assert_eq!(count(), 2);
/// # }).dispose();
/// #
/// ```
pub fn create_rw_signal<T>(cx: Scope, value: T) -> RwSignal<T> {
    let s = cx.runtime.create_rw_signal(value);
    cx.with_scope_property(|prop| prop.push(ScopeProperty::Signal(s.id)));
    s
}

/// A signal that combines the getter and setter into one value, rather than
/// separating them into a [ReadSignal] and a [WriteSignal]. You may prefer this
/// its style, or it may be easier to pass around in a context or as a function argument.
/// ```
/// # use leptos_reactive::*;
/// # create_scope(create_runtime(), |cx| {
/// let count = create_rw_signal(cx, 0);
///
/// // ✅ set the value
/// count.set(1);
/// assert_eq!(count(), 1);
///
/// // ❌ don't try to call the getter within the setter
/// // count.set(count.get() + 1);
///
/// // ✅ instead, use .update() to mutate the value in place
/// count.update(|count: &mut i32| *count += 1);
/// assert_eq!(count(), 2);
/// # }).dispose();
/// #
/// ```
#[derive(Debug, PartialEq, Eq, Hash)]
pub struct RwSignal<T>
where
    T: 'static,
{
    pub(crate) runtime: RuntimeId,
    pub(crate) id: SignalId,
    pub(crate) ty: PhantomData<T>,
}

impl<T> Clone for RwSignal<T> {
    fn clone(&self) -> Self {
        Self {
            runtime: self.runtime,
            id: self.id,
            ty: self.ty,
        }
    }
}

impl<T> Copy for RwSignal<T> {}

impl<T> UntrackedGettableSignal<T> for RwSignal<T> {
    fn get_untracked(&self) -> T
    where
        T: Clone,
    {
        self.id
            .with_no_subscription(self.runtime, |v: &T| v.clone())
    }

    fn with_untracked<O>(&self, f: impl FnOnce(&T) -> O) -> O {
        self.id.with_no_subscription(self.runtime, f)
    }
}

impl<T> UntrackedSettableSignal<T> for RwSignal<T> {
    fn set_untracked(&self, new_value: T) {
        self.id
            .update_with_no_effect(self.runtime, |v| *v = new_value);
    }

    fn update_untracked(&self, f: impl FnOnce(&mut T)) {
        self.id.update_with_no_effect(self.runtime, f);
    }

    fn update_returning_untracked<U>(&self, f: impl FnOnce(&mut T) -> U) -> Option<U> {
        self.id.update_with_no_effect(self.runtime, f)
    }
}

impl<T> RwSignal<T>
where
    T: 'static,
{
    /// Applies a function to the current value of the signal, and subscribes
    /// the running effect to this signal.
    /// ```
    /// # use leptos_reactive::*;
    /// # create_scope(create_runtime(), |cx| {
    /// let name = create_rw_signal(cx, "Alice".to_string());
    ///
    /// // ❌ unnecessarily clones the string
    /// let first_char = move || name().chars().next().unwrap();
    /// assert_eq!(first_char(), 'A');
    ///
    /// // ✅ gets the first char without cloning the `String`
    /// let first_char = move || name.with(|n| n.chars().next().unwrap());
    /// assert_eq!(first_char(), 'A');
    /// name.set("Bob".to_string());
    /// assert_eq!(first_char(), 'B');
    /// # }).dispose();
    /// #
    /// ```
    pub fn with<U>(&self, f: impl FnOnce(&T) -> U) -> U {
        self.id.with(self.runtime, f)
    }

    /// Clones and returns the current value of the signal, and subscribes
    /// the running effect to this signal.
    /// ```
    /// # use leptos_reactive::*;
    /// # create_scope(create_runtime(), |cx| {
    /// let count = create_rw_signal(cx, 0);
    ///
    /// assert_eq!(count.get(), 0);
    ///
    /// // count() is shorthand for count.get()
    /// assert_eq!(count(), 0);
    /// # }).dispose();
    /// #
    /// ```
    pub fn get(&self) -> T
    where
        T: Clone,
    {
        self.id.with(self.runtime, T::clone)
    }

    /// Applies a function to the current value to mutate it in place
    /// and notifies subscribers that the signal has changed.
    /// ```
    /// # use leptos_reactive::*;
    /// # create_scope(create_runtime(), |cx| {
    /// let count = create_rw_signal(cx, 0);
    ///
    /// // notifies subscribers
    /// count.update(|n| *n = 1); // it's easier just to call set_count(1), though!
    /// assert_eq!(count(), 1);
    ///
    /// // you can include arbitrary logic in this update function
    /// // also notifies subscribers, even though the value hasn't changed
    /// count.update(|n| if *n > 3 { *n += 1 });
    /// assert_eq!(count(), 1);
    /// # }).dispose();
    /// ```
    pub fn update(&self, f: impl FnOnce(&mut T)) {
        self.id.update(self.runtime, f);
    }

    /// Applies a function to the current value to mutate it in place
    /// and notifies subscribers that the signal has changed.
    /// Forwards the return value of the closure if the closure was called
    ///
    /// ```
    /// # use leptos_reactive::*;
    /// # create_scope(create_runtime(), |cx| {
    /// let count = create_rw_signal(cx, 0);
    ///
    /// // notifies subscribers
    /// let value = count.update_returning(|n| { *n = 1; *n * 10 });
    /// assert_eq!(value, Some(10));
    /// assert_eq!(count(), 1);
    ///
    /// let value = count.update_returning(|n| { *n += 1; *n * 10 });
    /// assert_eq!(value, Some(20));
    /// assert_eq!(count(), 2);
    /// # }).dispose();
    /// ```
    pub fn update_returning<U>(&self, f: impl FnOnce(&mut T) -> U) -> Option<U> {
        self.id.update(self.runtime, f)
    }

    /// Sets the signal’s value and notifies subscribers.
    ///
    /// **Note:** `set()` does not auto-memoize, i.e., it will notify subscribers
    /// even if the value has not actually changed.
    /// ```
    /// # use leptos_reactive::*;
    /// # create_scope(create_runtime(), |cx| {
    /// let count = create_rw_signal(cx, 0);
    ///
    /// assert_eq!(count(), 0);
    /// count.set(1);
    /// assert_eq!(count(), 1);
    /// # }).dispose();
    /// ```
    pub fn set(&self, value: T) {
        self.id.update(self.runtime, |n| *n = value);
    }

    /// Returns a read-only handle to the signal.
    ///
    /// Useful if you're trying to give read access to another component but ensure that it can't write
    /// to the signal and cause other parts of the DOM to update.
    /// ```
    /// # use leptos_reactive::*;
    /// # create_scope(create_runtime(), |cx| {
    /// let count = create_rw_signal(cx, 0);
    /// let read_count = count.read_only();
    /// assert_eq!(count(), 0);
    /// assert_eq!(read_count(), 0);
    /// count.set(1);
    /// assert_eq!(count(), 1);
    /// assert_eq!(read_count(), 1);
    /// # }).dispose();
    /// ```
    pub fn read_only(&self) -> ReadSignal<T> {
        ReadSignal {
            runtime: self.runtime,
            id: self.id,
            ty: PhantomData,
        }
    }

    /// Returns a write-only handle to the signal.
    ///
    /// Useful if you're trying to give write access to another component, or split an
    /// `RwSignal` into a [ReadSignal] and a [WriteSignal].
    /// ```
    /// # use leptos_reactive::*;
    /// # create_scope(create_runtime(), |cx| {
    /// let count = create_rw_signal(cx, 0);
    /// let set_count = count.write_only();
    /// assert_eq!(count(), 0);
    /// set_count(1);
    /// assert_eq!(count(), 1);
    /// # }).dispose();
    /// ```
    pub fn write_only(&self) -> WriteSignal<T> {
        WriteSignal {
            runtime: self.runtime,
            id: self.id,
            ty: PhantomData,
        }
    }

    /// Splits an `RwSignal` into its getter and setter.
    /// ```
    /// # use leptos_reactive::*;
    /// # create_scope(create_runtime(), |cx| {
    /// let count = create_rw_signal(cx, 0);
    /// let (get_count, set_count) = count.split();
    /// assert_eq!(count(), 0);
    /// assert_eq!(get_count(), 0);
    /// set_count(1);
    /// assert_eq!(count(), 1);
    /// assert_eq!(get_count(), 1);
    /// # }).dispose();
    /// ```
    pub fn split(&self) -> (ReadSignal<T>, WriteSignal<T>) {
        (
            ReadSignal {
                runtime: self.runtime,
                id: self.id,
                ty: PhantomData,
            },
            WriteSignal {
                runtime: self.runtime,
                id: self.id,
                ty: PhantomData,
            },
        )
    }

    /// Generates a [Stream] that emits the new value of the signal whenever it changes.
    pub fn to_stream(&self) -> impl Stream<Item = T>
    where
        T: Clone,
    {
        self.read_only().to_stream()
    }
}

#[cfg(not(feature = "stable"))]
impl<T> FnOnce<()> for RwSignal<T>
where
    T: Clone,
{
    type Output = T;

    extern "rust-call" fn call_once(self, _args: ()) -> Self::Output {
        self.get()
    }
}

#[cfg(not(feature = "stable"))]
impl<T> FnMut<()> for RwSignal<T>
where
    T: Clone,
{
    extern "rust-call" fn call_mut(&mut self, _args: ()) -> Self::Output {
        self.get()
    }
}

#[cfg(not(feature = "stable"))]
impl<T> Fn<()> for RwSignal<T>
where
    T: Clone,
{
    extern "rust-call" fn call(&self, _args: ()) -> Self::Output {
        self.get()
    }
}

// Internals
slotmap::new_key_type! {
    /// Unique ID assigned to a signal.
    pub struct SignalId;
}

#[derive(Debug, Error)]
pub(crate) enum SignalError {
    #[error("tried to access a signal that had been disposed")]
    Disposed,
    #[error("error casting signal to type {0}")]
    Type(&'static str),
}

impl SignalId {
    pub(crate) fn subscribe(&self, runtime: &Runtime) {
        // add subscriber
        if let Some(observer) = runtime.observer.get() {
            let mut subs = runtime.signal_subscribers.borrow_mut();
            if let Some(subs) = subs.entry(*self) {
                subs.or_default().borrow_mut().insert(observer);
            }
        }
    }

    pub(crate) fn try_with_no_subscription<T, U>(
        &self,
        runtime: &Runtime,
        f: impl FnOnce(&T) -> U,
    ) -> Result<U, SignalError>
    where
        T: 'static,
    {
        // get the value
        let value = {
            let signals = runtime.signals.borrow();
            match signals.get(*self).cloned().ok_or(SignalError::Disposed) {
                Ok(s) => Ok(s),
                Err(e) => {
                    debug_warn!("[Signal::try_with] {e}");
                    Err(e)
                }
            }
        }?;
        let value = value.try_borrow().unwrap_or_else(|e| {
            debug_warn!(
                "Signal::try_with_no_subscription failed on Signal<{}>. It seems you're trying to read the value of a signal within an effect caused by updating the signal.",
                std::any::type_name::<T>()
            );
            panic!("{e}");
        });
        let value = value
            .downcast_ref::<T>()
            .ok_or_else(|| SignalError::Type(std::any::type_name::<T>()))?;
        Ok(f(value))
    }

    pub(crate) fn try_with<T, U>(
        &self,
        runtime: &Runtime,
        f: impl FnOnce(&T) -> U,
    ) -> Result<U, SignalError>
    where
        T: 'static,
    {
        self.subscribe(runtime);

        self.try_with_no_subscription(runtime, f)
    }

    pub(crate) fn with_no_subscription<T, U>(
        &self,
        runtime: RuntimeId,
        f: impl FnOnce(&T) -> U,
    ) -> U
    where
        T: 'static,
    {
        with_runtime(runtime, |runtime| {
            self.try_with_no_subscription(runtime, f).unwrap()
        })
    }

    pub(crate) fn with<T, U>(&self, runtime: RuntimeId, f: impl FnOnce(&T) -> U) -> U
    where
        T: 'static,
    {
        with_runtime(runtime, |runtime| self.try_with(runtime, f).unwrap())
    }

    fn update_value<T, U>(&self, runtime: RuntimeId, f: impl FnOnce(&mut T) -> U) -> Option<U>
    where
        T: 'static,
    {
        with_runtime(runtime, |runtime| {
            let value = {
                let signals = runtime.signals.borrow();
                signals.get(*self).cloned()
            };
            if let Some(value) = value {
                let mut value = value.borrow_mut();
                if let Some(value) = value.downcast_mut::<T>() {
                    Some(f(value))
                } else {
                    debug_warn!(
                        "[Signal::update] failed when downcasting to Signal<{}>",
                        std::any::type_name::<T>()
                    );
                    None
                }
            } else {
                debug_warn!(
                    "[Signal::update] You’re trying to update a Signal<{}> that has already been disposed of. This is probably either a logic error in a component that creates and disposes of scopes, or a Resource resolving after its scope has been dropped without having been cleaned up.",
                    std::any::type_name::<T>()
                );
                None
            }
        })
    }

    pub(crate) fn update<T, U>(
        &self,
        runtime_id: RuntimeId,
        f: impl FnOnce(&mut T) -> U,
    ) -> Option<U>
    where
        T: 'static,
    {
        with_runtime(runtime_id, |runtime| {
            // update the value
            let updated = self.update_value(runtime_id, f);

            // notify subscribers
            if updated.is_some() {
                let subs = {
                    let subs = runtime.signal_subscribers.borrow();
                    let subs = subs.get(*self);
                    subs.map(|subs| subs.borrow().clone())
                };
                if let Some(subs) = subs {
                    for sub in subs {
                        let effect = {
                            let effects = runtime.effects.borrow();
                            effects.get(sub).cloned()
                        };
                        if let Some(effect) = effect {
                            effect.run(sub, runtime_id);
                        }
                    }
                }
            };
            updated
        })
    }

    pub(crate) fn update_with_no_effect<T, U>(
        &self,
        runtime: RuntimeId,
        f: impl FnOnce(&mut T) -> U,
    ) -> Option<U>
    where
        T: 'static,
    {
        // update the value
        self.update_value(runtime, f)
    }
}
