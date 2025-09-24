//! Callbacks define a standard way to store functions and closures. They are useful
//! for component properties, because they can be used to define optional callback functions,
//! which generic props don’t support.
//!
//! The callback types implement [`Copy`], so they can easily be moved into and out of other closures, just like signals.
//!
//! # Types
//! This modules implements 2 callback types:
//! - [`Callback`](reactive_graph::callback::Callback)
//! - [`UnsyncCallback`](reactive_graph::callback::UnsyncCallback)
//!
//! Use `SyncCallback` if the function is not `Sync` and `Send`.

use crate::{
    owner::{LocalStorage, StoredValue},
    traits::{Dispose, WithValue},
    IntoReactiveValue,
};
use std::{fmt, rc::Rc, sync::Arc};

/// A wrapper trait for calling callbacks.
pub trait Callable<In: 'static, Out: 'static = ()> {
    /// calls the callback with the specified argument.
    ///
    /// Returns None if the callback has been disposed
    fn try_run(&self, input: In) -> Option<Out>;
    /// calls the callback with the specified argument.
    ///
    /// # Panics
    /// Panics if you try to run a callback that has been disposed
    fn run(&self, input: In) -> Out;
}

/// A callback type that is not required to be [`Send`] or [`Sync`].
///
/// # Example
/// ```
/// # use reactive_graph::prelude::*; use reactive_graph::callback::*;  let owner = reactive_graph::owner::Owner::new(); owner.set();
/// let _: UnsyncCallback<()> = UnsyncCallback::new(|_| {});
/// let _: UnsyncCallback<(i32, i32)> = (|_x: i32, _y: i32| {}).into();
/// let cb: UnsyncCallback<i32, String> = UnsyncCallback::new(|x: i32| x.to_string());
/// assert_eq!(cb.run(42), "42".to_string());
/// ```
pub struct UnsyncCallback<In: 'static, Out: 'static = ()>(
    StoredValue<Rc<dyn Fn(In) -> Out>, LocalStorage>,
);

impl<In> fmt::Debug for UnsyncCallback<In> {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        fmt.write_str("Callback")
    }
}

impl<In, Out> Copy for UnsyncCallback<In, Out> {}

impl<In, Out> Clone for UnsyncCallback<In, Out> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<In, Out> Dispose for UnsyncCallback<In, Out> {
    fn dispose(self) {
        self.0.dispose();
    }
}

impl<In, Out> UnsyncCallback<In, Out> {
    /// Creates a new callback from the given function.
    pub fn new<F>(f: F) -> UnsyncCallback<In, Out>
    where
        F: Fn(In) -> Out + 'static,
    {
        Self(StoredValue::new_local(Rc::new(f)))
    }

    /// Returns `true` if both callbacks wrap the same underlying function pointer.
    #[inline]
    pub fn matches(&self, other: &Self) -> bool {
        self.0.with_value(|self_value| {
            other
                .0
                .with_value(|other_value| Rc::ptr_eq(self_value, other_value))
        })
    }
}

impl<In: 'static, Out: 'static> Callable<In, Out> for UnsyncCallback<In, Out> {
    fn try_run(&self, input: In) -> Option<Out> {
        self.0.try_with_value(|fun| fun(input))
    }

    fn run(&self, input: In) -> Out {
        self.0.with_value(|fun| fun(input))
    }
}

macro_rules! impl_unsync_callable_from_fn {
    ($($arg:ident),*) => {
        impl<F, $($arg,)* T, Out> From<F> for UnsyncCallback<($($arg,)*), Out>
        where
            F: Fn($($arg),*) -> T + 'static,
            T: Into<Out> + 'static,
            $($arg: 'static,)*
        {
            fn from(f: F) -> Self {
                paste::paste!(
                    Self::new(move |($([<$arg:lower>],)*)| f($([<$arg:lower>]),*).into())
                )
            }
        }
    };
}

impl_unsync_callable_from_fn!();
impl_unsync_callable_from_fn!(P1);
impl_unsync_callable_from_fn!(P1, P2);
impl_unsync_callable_from_fn!(P1, P2, P3);
impl_unsync_callable_from_fn!(P1, P2, P3, P4);
impl_unsync_callable_from_fn!(P1, P2, P3, P4, P5);
impl_unsync_callable_from_fn!(P1, P2, P3, P4, P5, P6);
impl_unsync_callable_from_fn!(P1, P2, P3, P4, P5, P6, P7);
impl_unsync_callable_from_fn!(P1, P2, P3, P4, P5, P6, P7, P8);
impl_unsync_callable_from_fn!(P1, P2, P3, P4, P5, P6, P7, P8, P9);
impl_unsync_callable_from_fn!(P1, P2, P3, P4, P5, P6, P7, P8, P9, P10);
impl_unsync_callable_from_fn!(P1, P2, P3, P4, P5, P6, P7, P8, P9, P10, P11);
impl_unsync_callable_from_fn!(
    P1, P2, P3, P4, P5, P6, P7, P8, P9, P10, P11, P12
);

/// A callback type that is [`Send`] + [`Sync`].
///
/// # Example
/// ```
/// # use reactive_graph::prelude::*; use reactive_graph::callback::*;  let owner = reactive_graph::owner::Owner::new(); owner.set();
/// let _: Callback<()> = Callback::new(|_| {});
/// let _: Callback<(i32, i32)> = (|_x: i32, _y: i32| {}).into();
/// let cb: Callback<i32, String> = Callback::new(|x: i32| x.to_string());
/// assert_eq!(cb.run(42), "42".to_string());
/// ```
pub struct Callback<In, Out = ()>(
    StoredValue<Arc<dyn Fn(In) -> Out + Send + Sync>>,
)
where
    In: 'static,
    Out: 'static;

impl<In, Out> fmt::Debug for Callback<In, Out> {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        fmt.write_str("SyncCallback")
    }
}

impl<In, Out> Callable<In, Out> for Callback<In, Out> {
    fn try_run(&self, input: In) -> Option<Out> {
        self.0.try_with_value(|fun| fun(input))
    }

    fn run(&self, input: In) -> Out {
        self.0.with_value(|f| f(input))
    }
}

impl<In, Out> Clone for Callback<In, Out> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<In, Out> Dispose for Callback<In, Out> {
    fn dispose(self) {
        self.0.dispose();
    }
}

impl<In, Out> Copy for Callback<In, Out> {}

macro_rules! impl_callable_from_fn {
    ($($arg:ident),*) => {
        impl<F, $($arg,)* T, Out> From<F> for Callback<($($arg,)*), Out>
        where
            F: Fn($($arg),*) -> T + Send + Sync + 'static,
            T: Into<Out> + 'static,
            $($arg: Send + Sync + 'static,)*
        {
            fn from(f: F) -> Self {
                paste::paste!(
                    Self::new(move |($([<$arg:lower>],)*)| f($([<$arg:lower>]),*).into())
                )
            }
        }
    };
}

impl_callable_from_fn!();
impl_callable_from_fn!(P1);
impl_callable_from_fn!(P1, P2);
impl_callable_from_fn!(P1, P2, P3);
impl_callable_from_fn!(P1, P2, P3, P4);
impl_callable_from_fn!(P1, P2, P3, P4, P5);
impl_callable_from_fn!(P1, P2, P3, P4, P5, P6);
impl_callable_from_fn!(P1, P2, P3, P4, P5, P6, P7);
impl_callable_from_fn!(P1, P2, P3, P4, P5, P6, P7, P8);
impl_callable_from_fn!(P1, P2, P3, P4, P5, P6, P7, P8, P9);
impl_callable_from_fn!(P1, P2, P3, P4, P5, P6, P7, P8, P9, P10);
impl_callable_from_fn!(P1, P2, P3, P4, P5, P6, P7, P8, P9, P10, P11);
impl_callable_from_fn!(P1, P2, P3, P4, P5, P6, P7, P8, P9, P10, P11, P12);

impl<In: 'static, Out: 'static> Callback<In, Out> {
    /// Creates a new callback from the given function.
    #[track_caller]
    pub fn new<F>(fun: F) -> Self
    where
        F: Fn(In) -> Out + Send + Sync + 'static,
    {
        Self(StoredValue::new(Arc::new(fun)))
    }

    /// Returns `true` if both callbacks wrap the same underlying function pointer.
    #[inline]
    pub fn matches(&self, other: &Self) -> bool {
        self.0
            .try_with_value(|self_value| {
                other.0.try_with_value(|other_value| {
                    Arc::ptr_eq(self_value, other_value)
                })
            })
            .flatten()
            .unwrap_or(false)
    }
}

#[doc(hidden)]
pub struct __IntoReactiveValueMarkerCallbackSingleParam;

#[doc(hidden)]
pub struct __IntoReactiveValueMarkerCallbackStrOutputToString;

impl<I, O, F>
    IntoReactiveValue<
        Callback<I, O>,
        __IntoReactiveValueMarkerCallbackSingleParam,
    > for F
where
    F: Fn(I) -> O + Send + Sync + 'static,
{
    #[track_caller]
    fn into_reactive_value(self) -> Callback<I, O> {
        Callback::new(self)
    }
}

impl<I, O, F>
    IntoReactiveValue<
        UnsyncCallback<I, O>,
        __IntoReactiveValueMarkerCallbackSingleParam,
    > for F
where
    F: Fn(I) -> O + 'static,
{
    #[track_caller]
    fn into_reactive_value(self) -> UnsyncCallback<I, O> {
        UnsyncCallback::new(self)
    }
}

impl<I, F>
    IntoReactiveValue<
        Callback<I, String>,
        __IntoReactiveValueMarkerCallbackStrOutputToString,
    > for F
where
    F: Fn(I) -> &'static str + Send + Sync + 'static,
{
    #[track_caller]
    fn into_reactive_value(self) -> Callback<I, String> {
        Callback::new(move |i| self(i).to_string())
    }
}

impl<I, F>
    IntoReactiveValue<
        UnsyncCallback<I, String>,
        __IntoReactiveValueMarkerCallbackStrOutputToString,
    > for F
where
    F: Fn(I) -> &'static str + 'static,
{
    #[track_caller]
    fn into_reactive_value(self) -> UnsyncCallback<I, String> {
        UnsyncCallback::new(move |i| self(i).to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::Callable;
    use crate::{
        callback::{Callback, UnsyncCallback},
        owner::Owner,
        traits::Dispose,
        IntoReactiveValue,
    };

    struct NoClone {}

    #[test]
    fn clone_callback() {
        let owner = Owner::new();
        owner.set();

        let callback = Callback::new(move |_no_clone: NoClone| NoClone {});
        let _cloned = callback;
    }

    #[test]
    fn clone_unsync_callback() {
        let owner = Owner::new();
        owner.set();

        let callback =
            UnsyncCallback::new(move |_no_clone: NoClone| NoClone {});
        let _cloned = callback;
    }

    #[test]
    fn runback_from() {
        let owner = Owner::new();
        owner.set();

        let _callback: Callback<(), String> = (|| "test").into();
        let _callback: Callback<(i32, String), String> =
            (|num, s| format!("{num} {s}")).into();
        // Single params should work without needing the (foo,) tuple using IntoReactiveValue:
        let _callback: Callback<usize, &'static str> =
            (|_usize| "test").into_reactive_value();
        let _callback: Callback<usize, String> =
            (|_usize| "test").into_reactive_value();
    }

    #[test]
    fn sync_callback_from() {
        let owner = Owner::new();
        owner.set();

        let _callback: UnsyncCallback<(), String> = (|| "test").into();
        let _callback: UnsyncCallback<(i32, String), String> =
            (|num, s| format!("{num} {s}")).into();
        // Single params should work without needing the (foo,) tuple using IntoReactiveValue:
        let _callback: UnsyncCallback<usize, &'static str> =
            (|_usize| "test").into_reactive_value();
        let _callback: UnsyncCallback<usize, String> =
            (|_usize| "test").into_reactive_value();
    }

    #[test]
    fn sync_callback_try_run() {
        let owner = Owner::new();
        owner.set();

        let callback = Callback::new(move |arg| arg);
        assert_eq!(callback.try_run((0,)), Some((0,)));
        callback.dispose();
        assert_eq!(callback.try_run((0,)), None);
    }

    #[test]
    fn unsync_callback_try_run() {
        let owner = Owner::new();
        owner.set();

        let callback = UnsyncCallback::new(move |arg| arg);
        assert_eq!(callback.try_run((0,)), Some((0,)));
        callback.dispose();
        assert_eq!(callback.try_run((0,)), None);
    }

    #[test]
    fn callback_matches_same() {
        let owner = Owner::new();
        owner.set();

        let callback1 = Callback::new(|x: i32| x * 2);
        let callback2 = callback1;
        assert!(callback1.matches(&callback2));
    }

    #[test]
    fn callback_matches_different() {
        let owner = Owner::new();
        owner.set();

        let callback1 = Callback::new(|x: i32| x * 2);
        let callback2 = Callback::new(|x: i32| x + 1);
        assert!(!callback1.matches(&callback2));
    }

    #[test]
    fn unsync_callback_matches_same() {
        let owner = Owner::new();
        owner.set();

        let callback1 = UnsyncCallback::new(|x: i32| x * 2);
        let callback2 = callback1;
        assert!(callback1.matches(&callback2));
    }

    #[test]
    fn unsync_callback_matches_different() {
        let owner = Owner::new();
        owner.set();

        let callback1 = UnsyncCallback::new(|x: i32| x * 2);
        let callback2 = UnsyncCallback::new(|x: i32| x + 1);
        assert!(!callback1.matches(&callback2));
    }
}
