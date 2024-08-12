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
//!     #[prop(into)] render_number: Callback<i32, String>,
//! ) -> impl IntoView {
//!     view! {
//!         <div>
//!             {render_number.run(1)}
//!             // callbacks can be called multiple times
//!             {render_number.run(42)}
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
//! - All callbacks implement the [`Callable`] trait, and can be invoked with `my_callback.run(input)`.
//! - The callback types implement [`Copy`], so they can easily be moved into and out of other closures, just like signals.
//!
//! # Types
//! This modules implements 2 callback types:
//! - [`Callback`]
//! - [`UnsyncCallback`]
//!
//! Use `SyncCallback` if the function is not `Sync` and `Send`.

use reactive_graph::owner::{LocalStorage, StoredValue};
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
}

impl<In: 'static, Out: 'static> Callable<In, Out> for UnsyncCallback<In, Out> {
    fn run(&self, input: In) -> Out {
        self.0.with_value(|fun| fun(input))
    }
}

impl<F, In, T, Out> From<F> for UnsyncCallback<In, Out>
where
    F: Fn(In) -> T + 'static,
    T: Into<Out> + 'static,
    In: 'static,
{
    fn from(f: F) -> Self {
        Self::new(move |x| f(x).into())
    }
}

/// Callbacks define a standard way to store functions and closures.
///
/// # Example
/// ```
/// # use leptos::prelude::*;
/// # use leptos::callback::{Callable, Callback};
/// #[component]
/// fn MyComponent(
///     #[prop(into)] render_number: Callback<i32, String>,
/// ) -> impl IntoView {
///     view! {
///         <div>
///             {render_number.run(42)}
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

impl<F, In, T, Out> From<F> for Callback<In, Out>
where
    F: Fn(In) -> T + Send + Sync + 'static,
    T: Into<Out> + 'static,
    In: Send + Sync + 'static,
{
    fn from(f: F) -> Self {
        Self::new(move |x| f(x).into())
    }
}

impl<In: 'static, Out: 'static> Callback<In, Out> {
    /// Creates a new callback from the given function.
    pub fn new<F>(fun: F) -> Self
    where
        F: Fn(In) -> Out + Send + Sync + 'static,
    {
        Self(StoredValue::new(Arc::new(fun)))
    }
}

#[cfg(test)]
mod tests {
    use crate::callback::{Callback, UnsyncCallback};

    struct NoClone {}

    #[test]
    fn clone_callback() {
        let callback = Callback::new(move |_no_clone: NoClone| NoClone {});
        let _cloned = callback.clone();
    }

    #[test]
    fn clone_unsync_callback() {
        let callback =
            UnsyncCallback::new(move |_no_clone: NoClone| NoClone {});
        let _cloned = callback.clone();
    }

    #[test]
    fn runback_from() {
        let _callback: Callback<(), String> = (|()| "test").into();
    }

    #[test]
    fn sync_callback_from() {
        let _callback: UnsyncCallback<(), String> = (|()| "test").into();
    }
}
