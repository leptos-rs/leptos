#![forbid(unsafe_code)]
use crate::{
    create_rw_signal, RwSignal, Scope, UntrackedGettableSignal, UntrackedRefSignal,
    UntrackedSettableSignal, UntrackedUpdatableSignal,
};

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

/// # Examples
///
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
impl<T: Clone> UntrackedGettableSignal<T> for StoredValue<T> {
    fn get_untracked(&self) -> T
    where
        T: Clone,
    {
        self.0.get_untracked()
    }
}

impl<T> UntrackedRefSignal<T> for StoredValue<T> {
    fn with_untracked<O>(&self, f: impl FnOnce(&T) -> O) -> O {
        self.0.with_untracked(f)
    }
}

impl<T> UntrackedSettableSignal<T> for StoredValue<T> {
    fn set_untracked(&self, new_value: T) {
        self.0.set_untracked(new_value)
    }
}

impl<T> StoredValue<T> {
    /// Returns a clone of the signals current value, subscribing the effect
    /// to this signal.
    #[track_caller]
    #[deprecated = "Please use `get_untracked` instead, as this method does not track the stored value. This method will also be removed in a future version of `leptos`"]
    pub fn get(&self) -> T
    where
        T: Clone,
    {
        self.get_untracked()
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
    #[track_caller]
    #[deprecated = "Please use `with_untracked` instead, as this method does not track the stored value. This method will also be removed in a future version of `leptos`"]
    pub fn with<U>(&self, f: impl FnOnce(&T) -> U) -> U {
        self.with_untracked(f)
    }

    /// Updates the stored value.
    #[track_caller]
    #[deprecated = "Please use `update_untracked` instead, as this method does not track the stored value. This method will also be removed in a future version of `leptos`"]
    pub fn update(&self, f: impl FnOnce(&mut T)) {
        self.update_untracked(f);
    }

    /// Updates the stored value.
    #[track_caller]
    #[deprecated = "Please use `try_update_untracked` instead, as this method does not track the stored value. This method will also be removed in a future version of `leptos`"]
    pub fn update_returning<U>(&self, f: impl FnOnce(&mut T) -> U) -> Option<U> {
        self.try_update_untracked(f)
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
    #[track_caller]
    #[deprecated = "Please use `set_untracked` instead, as this method does not track the stored value. This method will also be removed in a future version of `leptos`"]
    pub fn set(&self, value: T) {
        self.set_untracked(value);
    }
}

/// # Examples
///
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
///
/// ```
/// use leptos_reactive::*;
/// # create_scope(create_runtime(), |cx| {
///
///     pub struct MyUncloneableData {
///         pub value: String
///     }
///
///     let data = store_value(cx, MyUncloneableData { value: "a".into() });
///     let updated = data.update_returning(|data| {
///         data.value = "b".into();
///         data.value.clone()
///     });
///
///     assert_eq!(data.with(|data| data.value.clone()), "b");
///     assert_eq!(updated, Some(String::from("b")));
/// });
/// ```
impl<T> UntrackedUpdatableSignal<T> for StoredValue<T> {
    fn update_untracked(&self, f: impl FnOnce(&mut T)) {
        self.0.update_untracked(f)
    }

    fn update_returning_untracked<U>(&self, f: impl FnOnce(&mut T) -> U) -> Option<U> {
        self.0.try_update_untracked(f)
    }

    fn try_update_untracked<U>(&self, f: impl FnOnce(&mut T) -> U) -> Option<U> {
        self.0.try_update_untracked(f)
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

impl_get_fn_traits!(StoredValue);
