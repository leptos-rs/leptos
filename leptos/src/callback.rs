//! Callbacks define a standard way to store functions and closures. They are useful
//! for component properties, because they can be used to define optional callback functions,
//! which generic props don’t support.
//!
//! # Usage
//! Callbacks can be created manually from any function or closure, but the easiest way
//! to create them is to use `#[prop(into)]]` when defining a component.
//! ```
//! # use leptos::*;
//! #[component]
//! fn MyComponent(
//!     #[prop(into)] render_number: Callback<i32, String>,
//! ) -> impl IntoView {
//!     view! {
//!         <div>
//!             {render_number.call(1)}
//!             // callbacks can be called multiple times
//!             {render_number.call(42)}
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
//! - All callbacks implement the [`Callable`] trait, and can be invoked with `my_callback.call(input)`. On nightly, you can even do `my_callback(input)`
//! - The callback types implement [`Copy`], so they can easily be moved into and out of other closures, just like signals.
//!
//! # Types
//! This modules implements 2 callback types:
//! - [`Callback`]
//! - [`SyncCallback`]
//!
//! Use `SyncCallback` when you want the function to be `Sync` and `Send`.

#![cfg_attr(feature = "nightly", feature(fn_traits))]
#![cfg_attr(feature = "nightly", feature(unboxed_closures))]
#![cfg_attr(feature = "nightly", feature(auto_traits))]
#![cfg_attr(feature = "nightly", feature(negative_impls))]

use reactive_graph::owner::StoredValue;
use std::{fmt, rc::Rc, sync::Arc};

/// A wrapper trait for calling callbacks.
pub trait Callable<In: 'static, Out: 'static = ()> {
    /// calls the callback with the specified argument.
    fn call(&self, input: In) -> Out;
}

/// Callbacks define a standard way to store functions and closures.
///
/// # Example
/// ```
/// # use leptos::*;
/// # use leptos::{Callable, Callback};
/// #[component]
/// fn MyComponent(
///     #[prop(into)] render_number: Callback<i32, String>,
/// ) -> impl IntoView {
///     view! {
///         <div>
///             {render_number.call(42)}
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

pub struct UnsyncCallback<In: 'static, Out: 'static = ()>(
    Rc<dyn Fn(In) -> Out>,
);

impl<In> fmt::Debug for UnsyncCallback<In> {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        fmt.write_str("Callback")
    }
}

impl<In, Out> Clone for UnsyncCallback<In, Out> {
    fn clone(&self) -> Self {
        Self(Rc::clone(&self.0))
    }
}

impl<In, Out> UnsyncCallback<In, Out> {
    /// Creates a new callback from the given function.
    pub fn new<F>(f: F) -> UnsyncCallback<In, Out>
    where
        F: Fn(In) -> Out + 'static,
    {
        Self(Rc::new(f))
    }
}

impl<In: 'static, Out: 'static> Callable<In, Out> for UnsyncCallback<In, Out> {
    fn call(&self, input: In) -> Out {
        (self.0)(input)
    }
}

macro_rules! impl_from_fn {
    ($ty:ident) => {
        #[cfg(not(feature = "nightly"))]
        impl<F, In, T, Out> From<F> for $ty<In, Out>
        where
            F: Fn(In) -> T + Send + Sync + 'static,
            T: Into<Out> + 'static,
            In: Send + Sync + 'static,
        {
            fn from(f: F) -> Self {
                Self::new(move |x| f(x).into())
            }
        }

        paste::paste! {
            #[cfg(feature = "nightly")]
            auto trait [<NotRaw $ty>] {}

            #[cfg(feature = "nightly")]
            impl<A, B> ![<NotRaw $ty>] for $ty<A, B> {}

            #[cfg(feature = "nightly")]
            impl<F, In, T, Out> From<F> for $ty<In, Out>
            where
                F: Fn(In) -> T + Send + Sync + [<NotRaw $ty>] + 'static,
                T: Into<Out> + 'static,
                In: Send + Sync + 'static
            {
                fn from(f: F) -> Self {
                    Self::new(move |x| f(x).into())
                }
            }
        }
    };
}

// TODO
//impl_from_fn!(UnsyncCallback);

#[cfg(feature = "nightly")]
impl<In, Out> FnOnce<(In,)> for UnsyncCallback<In, Out> {
    type Output = Out;

    extern "rust-call" fn call_once(self, args: (In,)) -> Self::Output {
        Callable::call(&self, args.0)
    }
}

#[cfg(feature = "nightly")]
impl<In, Out> FnMut<(In,)> for UnsyncCallback<In, Out> {
    extern "rust-call" fn call_mut(&mut self, args: (In,)) -> Self::Output {
        Callable::call(&*self, args.0)
    }
}

#[cfg(feature = "nightly")]
impl<In, Out> Fn<(In,)> for UnsyncCallback<In, Out> {
    extern "rust-call" fn call(&self, args: (In,)) -> Self::Output {
        Callable::call(self, args.0)
    }
}

// TODO update these docs to swap the two
/// A callback type that is `Send` and `Sync` if its input type is `Send` and `Sync`.
/// Otherwise, you can use exactly the way you use [`Callback`].
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
    fn call(&self, input: In) -> Out {
        self.0
            .with_value(|f| f(input))
            .expect("called a callback that has been disposed")
    }
}

impl<In, Out> Clone for Callback<In, Out> {
    fn clone(&self) -> Self {
        Self(self.0)
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

impl_from_fn!(Callback);

#[cfg(feature = "nightly")]
impl<In, Out> FnOnce<(In,)> for Callback<In, Out>
where
    In: Send + Sync + 'static,
    Out: 'static,
{
    type Output = Out;

    extern "rust-call" fn call_once(self, args: (In,)) -> Self::Output {
        Callable::call(&self, args.0)
    }
}

#[cfg(feature = "nightly")]
impl<In, Out> FnMut<(In,)> for Callback<In, Out>
where
    In: Send + Sync + 'static,
    Out: 'static,
{
    extern "rust-call" fn call_mut(&mut self, args: (In,)) -> Self::Output {
        Callable::call(&*self, args.0)
    }
}

#[cfg(feature = "nightly")]
impl<In, Out> Fn<(In,)> for Callback<In, Out>
where
    In: Send + Sync + 'static,
    Out: 'static,
{
    extern "rust-call" fn call(&self, args: (In,)) -> Self::Output {
        Callable::call(self, args.0)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        callback::{Callback, UnsyncCallback},
        create_runtime,
    };

    struct NoClone {}

    #[test]
    fn clone_callback() {
        let rt = create_runtime();
        let callback =
            UnsyncCallback::new(move |_no_clone: NoClone| NoClone {});
        let _cloned = callback.clone();
        rt.dispose();
    }

    #[test]
    fn clone_sync_callback() {
        let rt = create_runtime();
        let callback = Callback::new(move |_no_clone: NoClone| NoClone {});
        let _cloned = callback.clone();
        rt.dispose();
    }

    #[test]
    fn callback_from() {
        let rt = create_runtime();
        let _callback: UnsyncCallback<(), String> = (|()| "test").into();
        rt.dispose();
    }

    #[test]
    fn callback_from_html() {
        let rt = create_runtime();
        use leptos::{
            html::{AnyElement, HtmlElement},
            *,
        };

        let _callback: UnsyncCallback<String, HtmlElement<AnyElement>> =
            (|x: String| {
                view! {
                    <h1>{x}</h1>
                }
            })
            .into();
        rt.dispose();
    }

    #[test]
    fn sync_callback_from() {
        let rt = create_runtime();
        let _callback: Callback<(), String> = (|()| "test").into();
        rt.dispose();
    }

    #[test]
    fn sync_callback_from_html() {
        use leptos::{
            html::{AnyElement, HtmlElement},
            *,
        };

        let rt = create_runtime();

        let _callback: Callback<String, HtmlElement<AnyElement>> =
            (|x: String| {
                view! {
                    <h1>{x}</h1>
                }
            })
            .into();

        rt.dispose();
    }
}
