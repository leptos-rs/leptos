#![forbid(unsafe_code)]
use crate::{
    create_rw_signal, RwSignal, Scope, SignalGetUntracked, SignalSetUntrack,
    SignalUpdateUntracked, SignalWithUntracked,
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

// For when we're ready to migrate these over to the untracked traits ;)
/*
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
/// # });
/// ```
impl<T: Clone> SignalGetUntracked<T> for StoredValue<T> {
    fn get_untracked(&self) -> T
    where
        T: Clone,
    {
        self.0.get_untracked()
    }
}

impl<T> SignalWithUntracked<T> for StoredValue<T> {
    fn with_untracked<O>(&self, f: impl FnOnce(&T) -> O) -> O {
        self.0.with_untracked(f)
    }

    fn try_with_untracked<O>(&self, f: impl FnOnce(&T) -> O) -> Option<O> {
        self.0.try_with_untracked(f)
    }
}

impl<T> SignalSetUntrack<T> for StoredValue<T> {
    fn set_untracked(&self, new_value: T) {
        self.0.set_untracked(new_value)
    }

    fn try_set_untracked(&self, new_value: T) -> Option<T> {
        self.0.try_set_untracked(new_value)
    }
}

impl<T> SignalWith<T> for StoredValue<T> {
    fn with<O>(&self, f: impl FnOnce(&T) -> O) -> O {
        self.0.with(f)
    }

    fn try_with<O>(&self, f: impl FnOnce(&T) -> O) -> Option<O> {
        self.0.try_with(f)
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
/// # });
/// ```
impl<T> SignalUpdateUntracked<T> for StoredValue<T> {
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
*/

impl<T> StoredValue<T> {
    /// Returns a clone of the signals current value, subscribing the effect
    /// to this signal.
    #[track_caller]
    #[deprecated = "Please use `get_value` instead, as this method does not \
                    track the stored value. This method will also be removed \
                    in a future version of `leptos`"]
    pub fn get(&self) -> T
    where
        T: Clone,
    {
        self.get_value()
    }

    /// Returns a clone of the signals current value, subscribing the effect
    /// to this signal.
    #[track_caller]
    pub fn get_value(&self) -> T
    where
        T: Clone,
    {
        self.0.get_untracked()
    }

    /// Same as [`StoredValue::get`] but will not panic by default.
    #[track_caller]
    #[deprecated = "Please use `try_get_value` instead, as this method does \
                    not track the stored value. This method will also be \
                    removed in a future version of `leptos`"]
    pub fn try_get(&self) -> Option<T>
    where
        T: Clone,
    {
        self.try_get_value()
    }

    /// Same as [`StoredValue::get`] but will not panic by default.
    #[track_caller]
    pub fn try_get_value(&self) -> Option<T>
    where
        T: Clone,
    {
        self.0.try_get_untracked()
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
    #[deprecated = "Please use `with_value` instead, as this method does not \
                    track the stored value. This method will also be removed \
                    in a future version of `leptos`"]
    pub fn with<U>(&self, f: impl FnOnce(&T) -> U) -> U {
        self.with_value(f)
    }

    /// Applies a function to the current stored value.
    /// ```
    /// # use leptos_reactive::*;
    /// # create_scope(create_runtime(), |cx| {
    ///
    /// pub struct MyUncloneableData {
    ///     pub value: String,
    /// }
    /// let data = store_value(cx, MyUncloneableData { value: "a".into() });
    ///
    /// // calling .with() to extract the value
    /// assert_eq!(data.with(|data| data.value.clone()), "a");
    /// # });
    /// ```
    #[track_caller]
    //               track the stored value. This method will also be removed in \
    //               a future version of `leptos`"]
    pub fn with_value<U>(&self, f: impl FnOnce(&T) -> U) -> U {
        self.0.with_untracked(f)
    }

    /// Same as [`StoredValue::with`] but returns [`Some(O)]` only if
    /// the signal is still valid. [`None`] otherwise.
    #[deprecated = "Please use `try_with_value` instead, as this method does \
                    not track the stored value. This method will also be \
                    removed in a future version of `leptos`"]
    pub fn try_with<O>(&self, f: impl FnOnce(&T) -> O) -> Option<O> {
        self.try_with_value(f)
    }

    /// Same as [`StoredValue::with`] but returns [`Some(O)]` only if
    /// the signal is still valid. [`None`] otherwise.
    pub fn try_with_value<O>(&self, f: impl FnOnce(&T) -> O) -> Option<O> {
        self.0.try_with_untracked(f)
    }

    /// Updates the stored value.
    #[track_caller]
    #[deprecated = "Please use `update_value` instead, as this method does not \
                    track the stored value. This method will also be removed \
                    in a future version of `leptos`"]
    pub fn update(&self, f: impl FnOnce(&mut T)) {
        self.update_value(f);
    }

    /// Updates the stored value.
    #[track_caller]
    pub fn update_value(&self, f: impl FnOnce(&mut T)) {
        self.0.update_untracked(f);
    }

    /// Updates the stored value.
    #[track_caller]
    #[deprecated = "Please use `try_update_value` instead, as this method does \
                    not track the stored value. This method will also be \
                    removed in a future version of `leptos`"]
    pub fn update_returning<U>(
        &self,
        f: impl FnOnce(&mut T) -> U,
    ) -> Option<U> {
        self.try_update_value(f)
    }

    /// Same as [`Self::update`], but returns [`Some(O)`] if the
    /// signal is still valid, [`None`] otherwise.
    pub fn try_update_value<O>(self, f: impl FnOnce(&mut T) -> O) -> Option<O> {
        self.0.try_update_untracked(f)
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
    #[deprecated = "Please use `set_value` instead, as this method does not \
                    track the stored value. This method will also be removed \
                    in a future version of `leptos`"]
    pub fn set(&self, value: T) {
        self.set_value(value);
    }

    /// Sets the stored value.
    /// ```
    /// # use leptos_reactive::*;
    /// # create_scope(create_runtime(), |cx| {
    ///
    /// pub struct MyUncloneableData {
    ///     pub value: String,
    /// }
    /// let data = store_value(cx, MyUncloneableData { value: "a".into() });
    /// data.set(MyUncloneableData { value: "b".into() });
    /// assert_eq!(data.with(|data| data.value.clone()), "b");
    /// # });
    /// ```
    #[track_caller]
    pub fn set_value(&self, value: T) {
        self.0.set_untracked(value);
    }

    /// Same as [`Self::set`], but returns [`None`] if the signal is
    /// still valid, [`Some(T)`] otherwise.
    pub fn try_set_value(&self, value: T) -> Option<T> {
        self.0.try_set_untracked(value)
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
///     pub value: String,
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

impl_get_fn_traits!(StoredValue(get_value));
