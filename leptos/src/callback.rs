//! Callbacks define a standard way to store functions and closures. They are useful
//! for component properties, because they can be used to define optional callback functions,
//! which generic props don’t support.
//!
//! # Usage
//! Callbacks can be created manually from any function or closure, but the easiest way
//! to create them is to use `#[prop(into)]]` when defining a component.
//! ```
//! use leptos::prelude::*;
//!
//! #[component]
//! fn MyComponent(
//!     #[prop(into)] render_number: Callback<(i32,), String>,
//! ) -> impl IntoView {
//!     view! {
//!         <div>
//!             {render_number.run((1,))}
//!             // callbacks can be called multiple times
//!             {render_number.run((42,))}
//!         </div>
//!     }
//! }
//! // you can pass a closure directly as `render_number`
//! fn test() -> impl IntoView {
//!     view! {
//!         <MyComponent render_number=|x: i32| x.to_string()/>
//!     }
//! }
//! ```
//!
//! *Notes*:
//! - The `render_number` prop can receive any type that implements `Fn(i32) -> String`.
//! - Callbacks are most useful when you want optional generic props.
//! - All callbacks implement the [`Callable`](leptos::callback::Callable) trait, and can be invoked with `my_callback.run(input)`.
//! - The callback types implement [`Copy`], so they can easily be moved into and out of other closures, just like signals.
//!
//! # Types
//! This modules implements 2 callback types:
//! - [`Callback`](leptos::callback::Callback)
//! - [`UnsyncCallback`](leptos::callback::UnsyncCallback)
//!
//! Use `SyncCallback` if the function is not `Sync` and `Send`.

use reactive_graph::{
    owner::{LocalStorage, StoredValue},
    traits::WithValue,
};
use std::{fmt, rc::Rc, sync::Arc};

/// A wrapper trait for calling callbacks.
pub trait Callable<In: 'static, Out: 'static = ()> {
    /// calls the callback with the specified argument.
    fn run(&self, input: In) -> Out;
}

/// A callback type that is not required to be `Send + Sync`.
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

/// Callbacks define a standard way to store functions and closures.
///
/// # Example
/// ```
/// # use leptos::prelude::*;
/// # use leptos::callback::{Callable, Callback};
/// #[component]
/// fn MyComponent(
///     #[prop(into)] render_number: Callback<(i32,), String>,
/// ) -> impl IntoView {
///     view! {
///         <div>
///             {render_number.run((42,))}
///         </div>
///     }
/// }
///
/// fn test() -> impl IntoView {
///     view! {
///         <MyComponent render_number=move |x: i32| x.to_string()/>
///     }
/// }
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
    fn run(&self, input: In) -> Out {
        self.0
            .try_with_value(|f| f(input))
            .expect("called a callback that has been disposed")
    }
}

impl<In, Out> Clone for Callback<In, Out> {
    fn clone(&self) -> Self {
        *self
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

#[cfg(test)]
mod tests {
    use crate::callback::{Callback, UnsyncCallback};

    struct NoClone {}

    #[test]
    fn clone_callback() {
        let callback = Callback::new(move |_no_clone: NoClone| NoClone {});
        let _cloned = callback;
    }

    #[test]
    fn clone_unsync_callback() {
        let callback =
            UnsyncCallback::new(move |_no_clone: NoClone| NoClone {});
        let _cloned = callback;
    }

    #[test]
    fn runback_from() {
        let _callback: Callback<(), String> = (|| "test").into();
        let _callback: Callback<(i32, String), String> =
            (|num, s| format!("{num} {s}")).into();
    }

    #[test]
    fn sync_callback_from() {
        let _callback: UnsyncCallback<(), String> = (|| "test").into();
        let _callback: UnsyncCallback<(i32, String), String> =
            (|num, s| format!("{num} {s}")).into();
    }

    #[test]
    fn callback_matches_same() {
        let callback1 = Callback::new(|x: i32| x * 2);
        let callback2 = callback1;
        assert!(callback1.matches(&callback2));
    }

    #[test]
    fn callback_matches_different() {
        let callback1 = Callback::new(|x: i32| x * 2);
        let callback2 = Callback::new(|x: i32| x + 1);
        assert!(!callback1.matches(&callback2));
    }

    #[test]
    fn unsync_callback_matches_same() {
        let callback1 = UnsyncCallback::new(|x: i32| x * 2);
        let callback2 = callback1;
        assert!(callback1.matches(&callback2));
    }

    #[test]
    fn unsync_callback_matches_different() {
        let callback1 = UnsyncCallback::new(|x: i32| x * 2);
        let callback2 = UnsyncCallback::new(|x: i32| x + 1);
        assert!(!callback1.matches(&callback2));
    }
}
