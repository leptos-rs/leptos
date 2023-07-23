#![forbid(unsafe_code)]
use crate::{
    store_value, RwSignal, Scope, SignalSet, StoredValue, WriteSignal,
};

/// Helper trait for converting `Fn(T)` into [`SignalSetter<T>`].
pub trait IntoSignalSetter<T>: Sized {
    /// Consumes `self`, returning [`SignalSetter<T>`].
    fn mapped_signal_setter(self, cx: Scope) -> SignalSetter<T>;
}

impl<F, T> IntoSignalSetter<T> for F
where
    F: Fn(T) + 'static,
{
    fn mapped_signal_setter(self, cx: Scope) -> SignalSetter<T> {
        SignalSetter::map(cx, self)
    }
}

/// A wrapper for any kind of settable reactive signal: a [`WriteSignal`](crate::WriteSignal),
/// [`RwSignal`](crate::RwSignal), or closure that receives a value and sets a signal depending
/// on it.
///
/// This allows you to create APIs that take any kind of `SignalSetter<T>` as an argument,
/// rather than adding a generic `F: Fn(T)`. Values can be set with the same
/// function call or `set()`, API as other signals.
///
/// ## Core Trait Implementations
/// - [`.set()`](#impl-SignalSet<T>-for-SignalSetter<T>) (or calling the setter as a function)
///   sets the signal’s value, and notifies all subscribers that the signal’s value has changed.
///   to subscribe to the signal, and to re-run whenever the value of the signal changes.
///
/// ## Examples
/// ```rust
/// # use leptos_reactive::*;
/// # create_scope(create_runtime(), |cx| {
/// let (count, set_count) = create_signal(cx, 2);
/// let set_double_input = SignalSetter::map(cx, move |n| set_count.set(n * 2));
///
/// // this function takes any kind of signal setter
/// fn set_to_4(setter: &SignalSetter<i32>) {
///     // ✅ calling the signal sets the value
///     //    can be `setter(4)` on nightly
///     setter.set(4);
/// }
///
/// set_to_4(&set_count.into());
/// assert_eq!(count.get(), 4);
/// set_to_4(&set_double_input);
/// assert_eq!(count.get(), 8);
/// # });
/// ```
#[derive(Debug, PartialEq, Eq)]
pub struct SignalSetter<T>
where
    T: 'static,
{
    inner: SignalSetterTypes<T>,
    #[cfg(any(debug_assertions, feature = "ssr"))]
    defined_at: &'static std::panic::Location<'static>,
}

impl<T> Clone for SignalSetter<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: Default + 'static> Default for SignalSetter<T> {
    #[track_caller]
    fn default() -> Self {
        Self {
            inner: SignalSetterTypes::Default,
            #[cfg(any(debug_assertions, feature = "ssr"))]
            defined_at: std::panic::Location::caller(),
        }
    }
}

impl<T> Copy for SignalSetter<T> {}

impl<T> SignalSet<T> for SignalSetter<T> {
    fn set(&self, new_value: T) {
        match self.inner {
            SignalSetterTypes::Default => {}
            SignalSetterTypes::Write(w) => w.set(new_value),
            SignalSetterTypes::Mapped(_, s) => {
                s.with_value(|setter| setter(new_value))
            }
        }
    }

    fn try_set(&self, new_value: T) -> Option<T> {
        match self.inner {
            SignalSetterTypes::Default => Some(new_value),
            SignalSetterTypes::Write(w) => w.try_set(new_value),
            SignalSetterTypes::Mapped(_, s) => {
                let mut new_value = Some(new_value);

                let _ = s
                    .try_with_value(|setter| setter(new_value.take().unwrap()));

                new_value
            }
        }
    }
}

impl<T> SignalSetter<T>
where
    T: 'static,
{
    /// Wraps a signal-setting closure, i.e., any computation that sets one or more
    /// reactive signals.
    /// ```rust
    /// # use leptos_reactive::*;
    /// # create_scope(create_runtime(), |cx| {
    /// let (count, set_count) = create_signal(cx, 2);
    /// let set_double_count = SignalSetter::map(cx, move |n| set_count.set(n * 2));
    ///
    /// // this function takes any kind of signal setter
    /// fn set_to_4(setter: &SignalSetter<i32>) {
    ///     // ✅ calling the signal sets the value
    ///     //    can be `setter(4)` on nightly
    ///     setter.set(4)
    /// }
    ///
    /// set_to_4(&set_count.into());
    /// assert_eq!(count.get(), 4);
    /// set_to_4(&set_double_count);
    /// assert_eq!(count.get(), 8);
    /// # });
    /// ```
    #[track_caller]
    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(
            level = "trace",
            skip_all,
            fields(
                cx = ?cx.id,
            )
        )
    )]
    pub fn map(cx: Scope, mapped_setter: impl Fn(T) + 'static) -> Self {
        Self {
            inner: SignalSetterTypes::Mapped(
                cx,
                store_value(cx, Box::new(mapped_setter)),
            ),
            #[cfg(any(debug_assertions, feature = "ssr"))]
            defined_at: std::panic::Location::caller(),
        }
    }

    /// Calls the setter function with the given value.
    ///
    /// ```rust
    /// # use leptos_reactive::*;
    /// # create_scope(create_runtime(), |cx| {
    /// let (count, set_count) = create_signal(cx, 2);
    /// let set_double_count = SignalSetter::map(cx, move |n| set_count.set(n * 2));
    ///
    /// // this function takes any kind of signal setter
    /// fn set_to_4(setter: &SignalSetter<i32>) {
    ///   // ✅ calling the signal sets the value
    ///   //    can be `setter(4)` on nightly
    ///   setter.set(4);
    /// }
    ///
    /// set_to_4(&set_count.into());
    /// assert_eq!(count.get(), 4);
    /// set_to_4(&set_double_count);
    /// assert_eq!(count.get(), 8);
    /// # });
    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(
            level = "trace",
            skip_all,
            fields(
                defined_at = %self.defined_at,
                ty = %std::any::type_name::<T>()
            )
        )
    )]
    pub fn set(&self, value: T) {
        match &self.inner {
            SignalSetterTypes::Write(s) => s.set(value),
            SignalSetterTypes::Mapped(_, s) => s.with_value(|s| s(value)),
            SignalSetterTypes::Default => {}
        }
    }
}

impl<T> From<WriteSignal<T>> for SignalSetter<T> {
    #[track_caller]
    fn from(value: WriteSignal<T>) -> Self {
        Self {
            inner: SignalSetterTypes::Write(value),
            #[cfg(any(debug_assertions, feature = "ssr"))]
            defined_at: std::panic::Location::caller(),
        }
    }
}

impl<T> From<RwSignal<T>> for SignalSetter<T> {
    #[track_caller]
    fn from(value: RwSignal<T>) -> Self {
        Self {
            inner: SignalSetterTypes::Write(value.write_only()),
            #[cfg(any(debug_assertions, feature = "ssr"))]
            defined_at: std::panic::Location::caller(),
        }
    }
}

enum SignalSetterTypes<T>
where
    T: 'static,
{
    Write(WriteSignal<T>),
    Mapped(Scope, StoredValue<Box<dyn Fn(T)>>),
    Default,
}

impl<T> Clone for SignalSetterTypes<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for SignalSetterTypes<T> {}

impl<T> std::fmt::Debug for SignalSetterTypes<T>
where
    T: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Write(arg0) => {
                f.debug_tuple("WriteSignal").field(arg0).finish()
            }
            Self::Mapped(_, _) => f.debug_tuple("Mapped").finish(),
            Self::Default => f.debug_tuple("SignalSetter<Default>").finish(),
        }
    }
}

impl<T> PartialEq for SignalSetterTypes<T>
where
    T: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Write(l0), Self::Write(r0)) => l0 == r0,
            (Self::Mapped(_, l0), Self::Mapped(_, r0)) => std::ptr::eq(l0, r0),
            _ => false,
        }
    }
}

impl<T> Eq for SignalSetterTypes<T> where T: PartialEq {}

impl_set_fn_traits![SignalSetter];
