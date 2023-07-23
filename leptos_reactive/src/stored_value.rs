#![forbid(unsafe_code)]
use crate::{with_runtime, RuntimeId, Scope, ScopeProperty};
use std::{
    cell::RefCell,
    fmt,
    hash::{Hash, Hasher},
    marker::PhantomData,
    rc::Rc,
};

slotmap::new_key_type! {
    /// Unique ID assigned to a [`StoredValue`].
    pub(crate) struct StoredValueId;
}

/// A **non-reactive** wrapper for any value, which can be created with [`store_value`].
///
/// If you want a reactive wrapper, use [`create_signal`](crate::create_signal).
///
/// This allows you to create a stable reference for any value by storing it within
/// the reactive system. Like the signal types (e.g., [`ReadSignal`](crate::ReadSignal)
/// and [`RwSignal`](crate::RwSignal)), it is `Copy` and `'static`. Unlike the signal
/// types, it is not reactive; accessing it does not cause effects to subscribe, and
/// updating it does not notify anything else.
pub struct StoredValue<T>
where
    T: 'static,
{
    runtime: RuntimeId,
    id: StoredValueId,
    ty: PhantomData<T>,
}

impl<T> Clone for StoredValue<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for StoredValue<T> {}

impl<T> fmt::Debug for StoredValue<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StoredValue")
            .field("runtime", &self.runtime)
            .field("id", &self.id)
            .field("ty", &self.ty)
            .finish()
    }
}

impl<T> Eq for StoredValue<T> {}

impl<T> PartialEq for StoredValue<T> {
    fn eq(&self, other: &Self) -> bool {
        self.runtime == other.runtime && self.id == other.id
    }
}

impl<T> Hash for StoredValue<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.runtime.hash(state);
        self.id.hash(state);
    }
}

impl<T> StoredValue<T> {
    /// Returns a clone of the current stored value.
    ///
    /// # Panics
    /// Panics if you try to access a value stored in a [`Scope`] that has been disposed.
    ///
    /// # Examples
    /// ```
    /// # use leptos_reactive::*;
    /// # create_scope(create_runtime(), |cx| {
    ///
    /// #[derive(Clone)]
    /// pub struct MyCloneableData {
    ///     pub value: String,
    /// }
    /// let data = store_value(cx, MyCloneableData { value: "a".into() });
    ///
    /// // calling .get_value() clones and returns the value
    /// assert_eq!(data.get_value().value, "a");
    /// // can be `data().value` on nightly
    /// // assert_eq!(data().value, "a");
    /// # });
    /// ```
    #[track_caller]
    pub fn get_value(&self) -> T
    where
        T: Clone,
    {
        self.try_get_value().expect("could not get stored value")
    }

    /// Same as [`StoredValue::get_value`] but will not panic by default.
    #[track_caller]
    pub fn try_get_value(&self) -> Option<T>
    where
        T: Clone,
    {
        self.try_with_value(T::clone)
    }

    /// Applies a function to the current stored value and returns the result.
    ///
    /// # Panics
    /// Panics if you try to access a value stored in a [`Scope`] that has been disposed.
    ///
    /// # Examples
    /// ```
    /// # use leptos_reactive::*;
    /// # create_scope(create_runtime(), |cx| {
    ///
    /// pub struct MyUncloneableData {
    ///     pub value: String,
    /// }
    /// let data = store_value(cx, MyUncloneableData { value: "a".into() });
    ///
    /// // calling .with_value() to extract the value
    /// assert_eq!(data.with_value(|data| data.value.clone()), "a");
    /// # });
    /// ```
    #[track_caller]
    //               track the stored value. This method will also be removed in \
    //               a future version of `leptos`"]
    pub fn with_value<U>(&self, f: impl FnOnce(&T) -> U) -> U {
        self.try_with_value(f).expect("could not get stored value")
    }

    /// Same as [`StoredValue::with_value`] but returns [`Some(O)]` only if
    /// the stored value has not yet been disposed. [`None`] otherwise.
    pub fn try_with_value<O>(&self, f: impl FnOnce(&T) -> O) -> Option<O> {
        with_runtime(self.runtime, |runtime| {
            let value = {
                let values = runtime.stored_values.borrow();
                values.get(self.id)?.clone()
            };
            let value = value.borrow();
            let value = value.downcast_ref::<T>()?;
            Some(f(value))
        })
        .ok()
        .flatten()
    }

    /// Updates the stored value.
    ///
    /// # Examples
    /// ```
    /// # use leptos_reactive::*;
    /// # create_scope(create_runtime(), |cx| {
    ///
    /// pub struct MyUncloneableData {
    ///   pub value: String
    /// }
    /// let data = store_value(cx, MyUncloneableData { value: "a".into() });
    /// data.update_value(|data| data.value = "b".into());
    /// assert_eq!(data.with_value(|data| data.value.clone()), "b");
    /// });
    /// ```
    ///
    /// ```
    /// use leptos_reactive::*;
    /// # create_scope(create_runtime(), |cx| {
    ///
    /// pub struct MyUncloneableData {
    ///     pub value: String,
    /// }
    ///
    /// let data = store_value(cx, MyUncloneableData { value: "a".into() });
    /// let updated = data.try_update_value(|data| {
    ///     data.value = "b".into();
    ///     data.value.clone()
    /// });
    ///
    /// assert_eq!(data.with_value(|data| data.value.clone()), "b");
    /// assert_eq!(updated, Some(String::from("b")));
    /// # });
    /// ```
    #[track_caller]
    pub fn update_value(&self, f: impl FnOnce(&mut T)) {
        self.try_update_value(f)
            .expect("could not set stored value");
    }

    /// Same as [`Self::update_value`], but returns [`Some(O)`] if the
    /// stored value has not yet been disposed, [`None`] otherwise.
    pub fn try_update_value<O>(self, f: impl FnOnce(&mut T) -> O) -> Option<O> {
        with_runtime(self.runtime, |runtime| {
            let value = {
                let values = runtime.stored_values.borrow();
                values.get(self.id)?.clone()
            };
            let mut value = value.borrow_mut();
            let value = value.downcast_mut::<T>()?;
            Some(f(value))
        })
        .ok()
        .flatten()
    }

    /// Sets the stored value.
    ///
    /// # Examples
    /// ```
    /// # use leptos_reactive::*;
    /// # create_scope(create_runtime(), |cx| {
    ///
    /// pub struct MyUncloneableData {
    ///     pub value: String,
    /// }
    /// let data = store_value(cx, MyUncloneableData { value: "a".into() });
    /// data.set_value(MyUncloneableData { value: "b".into() });
    /// assert_eq!(data.with_value(|data| data.value.clone()), "b");
    /// # });
    /// ```
    #[track_caller]
    pub fn set_value(&self, value: T) {
        self.try_set_value(value);
    }

    /// Same as [`Self::set_value`], but returns [`None`] if the
    /// stored value has not yet been disposed, [`Some(T)`] otherwise.
    pub fn try_set_value(&self, value: T) -> Option<T> {
        with_runtime(self.runtime, |runtime| {
            let n = {
                let values = runtime.stored_values.borrow();
                values.get(self.id).map(Rc::clone)
            };

            if let Some(n) = n {
                let mut n = n.borrow_mut();
                let n = n.downcast_mut::<T>();
                if let Some(n) = n {
                    *n = value;
                    None
                } else {
                    Some(value)
                }
            } else {
                Some(value)
            }
        })
        .ok()
        .flatten()
    }
}

/// Creates a **non-reactive** wrapper for any value by storing it within
/// the reactive system.
///
/// Like the signal types (e.g., [`ReadSignal`](crate::ReadSignal)
/// and [`RwSignal`](crate::RwSignal)), it is `Copy` and `'static`. Unlike the signal
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
/// // ✅ you can move the `StoredValue` and access it with .with_value()
/// let data = store_value(cx, MyUncloneableData { value: "a".into() });
/// let callback_a = move || data.with_value(|data| data.value == "a");
/// let callback_b = move || data.with_value(|data| data.value == "b");
/// # }).dispose();
/// ```
#[track_caller]
pub fn store_value<T>(cx: Scope, value: T) -> StoredValue<T>
where
    T: 'static,
{
    let id = with_runtime(cx.runtime, |runtime| {
        runtime
            .stored_values
            .borrow_mut()
            .insert(Rc::new(RefCell::new(value)))
    })
    .unwrap_or_default();
    cx.push_scope_property(ScopeProperty::StoredValue(id));
    StoredValue {
        runtime: cx.runtime,
        id,
        ty: PhantomData,
    }
}

impl_get_fn_traits!(StoredValue(get_value));
