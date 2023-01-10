#![forbid(unsafe_code)]
use crate::{create_rw_signal, RwSignal, Scope, UntrackedGettableSignal, UntrackedSettableSignal};

/// A **non-reactive** wrapper for any value, which can be created with [store_value].
///
/// If you want a reactive wrapper, use [create_signal](crate::create_signal).
///
/// This allows you to create a stable reference for any value by storing it within
/// the reactive system. Like the signal types (e.g., [ReadSignal](crate::ReadSignal)
/// and [RwSignal](crate::RwSignal)), it is `Copy` and `'static`. Unlike the signal
/// types, it is not reactive; accessing it does not cause effects to subscribe, and
/// updating it does not notify anything else.
#[derive(Debug, PartialEq, Eq, Hash)]
pub struct StoredValue<T>(RwSignal<T>)
where
    T: 'static;

impl<T> Clone for StoredValue<T> {
    fn clone(&self) -> Self {
        Self(self.0)
    }
}

impl<T> Copy for StoredValue<T> {}

impl<T> StoredValue<T>
where
    T: 'static,
{
    /// Clones and returns the current stored value.
    ///
    /// If you want to get the value without cloning it, use [StoredValue::with].
    /// (`value.get()` is equivalent to `value.with(T::clone)`.)
    /// ```
    /// # use leptos_reactive::*;
    /// # create_scope(create_runtime(), |cx| {
    ///
    /// #[derive(Clone)]
    /// pub struct MyCloneableData {
    ///   pub value: String
    /// }
    /// let data = store_value(cx, MyCloneableData { value: "a".into() });
    ///
    /// // calling .get() clones and returns the value
    /// assert_eq!(data.get().value, "a");
    /// // there's a short-hand getter form
    /// assert_eq!(data().value, "a");
    /// });
    /// ```
    pub fn get(&self) -> T
    where
        T: Clone,
    {
        self.with(T::clone)
    }

    /// Applies a function to the current stored value.
    /// ```
    /// # use leptos_reactive::*;
    /// # create_scope(create_runtime(), |cx| {
    ///
    /// pub struct MyUncloneableData {
    ///   pub value: String
    /// }
    /// let data = store_value(cx, MyUncloneableData { value: "a".into() });
    ///
    /// // calling .with() to extract the value
    /// assert_eq!(data.with(|data| data.value.clone()), "a");
    /// });
    /// ```
    pub fn with<U>(&self, f: impl FnOnce(&T) -> U) -> U {
        self.0.with_untracked(f)
    }

    /// Applies a function to the current value to mutate it in place.
    /// ```
    /// # use leptos_reactive::*;
    /// # create_scope(create_runtime(), |cx| {
    ///
    /// pub struct MyUncloneableData {
    ///   pub value: String
    /// }
    /// let data = store_value(cx, MyUncloneableData { value: "a".into() });
    /// data.update(|data| data.value = "b".into());
    /// assert_eq!(data.with(|data| data.value.clone()), "b");
    /// });
    /// ```
    pub fn update(&self, f: impl FnOnce(&mut T)) {
        self.0.update_untracked(f);
    }

    /// Sets the stored value.
    /// ```
    /// # use leptos_reactive::*;
    /// # create_scope(create_runtime(), |cx| {
    ///
    /// pub struct MyUncloneableData {
    ///   pub value: String
    /// }
    /// let data = store_value(cx, MyUncloneableData { value: "a".into() });
    /// data.set(MyUncloneableData { value: "b".into() });
    /// assert_eq!(data.with(|data| data.value.clone()), "b");
    /// });
    /// ```
    pub fn set(&self, value: T) {
        self.0.set_untracked(value);
    }
}

/// Creates a **non-reactive** wrapper for any value by storing it within
/// the reactive system.
///
/// Like the signal types (e.g., [ReadSignal](crate::ReadSignal)
/// and [RwSignal](crate::RwSignal)), it is `Copy` and `'static`. Unlike the signal
/// types, it is not reactive; accessing it does not cause effects to subscribe, and
/// updating it does not notify anything else.
/// ```compile_fail
/// # use leptos_reactive::*;
/// # create_scope(create_runtime(), |cx| {
/// // this structure is neither `Copy` nor `Clone`
/// pub struct MyUncloneableData {
///   pub value: String
/// }
///
/// // ❌ this won't compile, as it can't be cloned or copied into the closures
/// let data = MyUncloneableData { value: "a".into() };
/// let callback_a = move || data.value == "a";
/// let callback_b = move || data.value == "b";
/// # }).dispose();
/// ```
/// ```
/// # use leptos_reactive::*;
/// # create_scope(create_runtime(), |cx| {
/// // this structure is neither `Copy` nor `Clone`
/// pub struct MyUncloneableData {
///   pub value: String
/// }
///
/// // ✅ you can move the `StoredValue` and access it with .with()
/// let data = store_value(cx, MyUncloneableData { value: "a".into() });
/// let callback_a = move || data.with(|data| data.value == "a");
/// let callback_b = move || data.with(|data| data.value == "b");
/// # }).dispose();
/// ```
pub fn store_value<T>(cx: Scope, value: T) -> StoredValue<T>
where
    T: 'static,
{
    StoredValue(create_rw_signal(cx, value))
}

#[cfg(not(feature = "stable"))]
impl<T> FnOnce<()> for StoredValue<T>
where
    T: Clone,
{
    type Output = T;

    extern "rust-call" fn call_once(self, _args: ()) -> Self::Output {
        self.get()
    }
}

#[cfg(not(feature = "stable"))]
impl<T> FnMut<()> for StoredValue<T>
where
    T: Clone,
{
    extern "rust-call" fn call_mut(&mut self, _args: ()) -> Self::Output {
        self.get()
    }
}

#[cfg(not(feature = "stable"))]
impl<T> Fn<()> for StoredValue<T>
where
    T: Clone,
{
    extern "rust-call" fn call(&self, _args: ()) -> Self::Output {
        self.get()
    }
}
