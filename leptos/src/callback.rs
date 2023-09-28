//! Callbacks define a standard way to store functions and closures. They are useful
//! for component properties, because they can be used to define optional callback functions,
//! which generic props don’t support.
//!
//! # Usage
//! Callbacks can be created manually from any function (including closures), but the easiest way
//! to create them is to use `#[prop(into)]]` when defining a component.
//! ```
//! # use leptos::*;
//! #[component]
//! fn MyComponent(
//!     #[prop(into)] render_number: Callback<i32, String>,
//! ) -> impl IntoView {
//!     view! {
//!         <div>
//!             {render_number.call(42)}
//!         </div>
//!     }
//! }
//! // now you can use it from a closure directly:
//! fn test() -> impl IntoView {
//!     view! {
//!         <MyComponent render_number=|x: i32| x.to_string()/>
//!     }
//! }
//! ```
//!
//! *Notes*:
//! - The `render_number` prop can receive any type that implements `Fn(i32) -> String`.
//!   Callbacks are most useful when you want optional generic props.
//! - All callbacks implement the [`Callable`] trait, and can be invoked with `my_callback.call(input)`.
//! - The callback types implement [`Clone`] but not [`Copy`]. If you want a callback that implements [`Copy`],
//!   you can use [`store_value`][leptos_reactive::store_value].
//! ```
//! # use leptos::*;
//! fn test() -> impl IntoView {
//!     let callback: Callback<i32, String> =
//!         Callback::new(|x: i32| x.to_string());
//!     let stored_callback = store_value(callback);
//!
//!     view! {
//!         <div>
//!             // `stored_callback` can be moved multiple times
//!             {move || stored_callback.call(1)}
//!             {move || stored_callback.call(42)}
//!         </div>
//!     }
//! }
//! ```
//!
//! Note that for each callback type `T`, `StoredValue<T>` implements `Call`, so you can call them
//! without even thinking about it.

use leptos_reactive::StoredValue;
use std::{fmt, rc::Rc, sync::Arc};

/// A wrapper trait for calling callbacks.
pub trait Callable<In, Out = ()> {
    /// calls the callback with the specified argument.
    fn call(&self, input: In) -> Out;
}

/// The most basic leptos callback type.
/// For how to use callbacks, see [here][crate::callback]
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
///
/// # Cloning
/// See [StoredCallback]

pub struct Callback<In, Out = ()>(Rc<dyn Fn(In) -> Out>);

impl<In> fmt::Debug for Callback<In> {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        fmt.write_str("Callback")
    }
}

impl<In, Out> Clone for Callback<In, Out> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<In, Out> Callback<In, Out> {
    /// Creates a new callback from the given function.
    pub fn new<F>(f: F) -> Callback<In, Out>
    where
        F: Fn(In) -> Out + 'static,
    {
        Self(Rc::new(f))
    }
}

impl<In, Out> Callable<In, Out> for Callback<In, Out> {
    fn call(&self, input: In) -> Out {
        (self.0)(input)
    }
}

#[cfg(not(feature = "nightly"))]
impl<F, In, T, Out> From<F> for Callback<In, Out>
where
    F: Fn(In) -> T + 'static,
    T: Into<Out> + 'static,
{
    fn from(f: F) -> Callback<In, Out> {
        Callback::new(move |x| f(x).into())
    }
}

// will allow to implement `Fn` for Callback in the future if needed.
#[cfg(feature = "nightly")]
auto trait NotRawCallback {}
#[cfg(feature = "nightly")]
impl<A, B> !NotRawCallback for Callback<A, B> {}
#[cfg(feature = "nightly")]
impl<F, In, T, Out> From<F> for Callback<In, Out>
where
    F: Fn(In) -> T + NotRawCallback + 'static,
    T: Into<Out> + 'static,
{
    fn from(f: F) -> Callback<In, Out> {
        Callback::new(move |x| f(x).into())
    }
}

#[cfg(feature = "nightly")]
impl<In, Out> FnOnce<(In,)> for Callback<In, Out> {
    type Output = Out;

    extern "rust-call" fn call_once(self, args: (In,)) -> Self::Output {
        Callable::call(&self, args.0)
    }
}

#[cfg(feature = "nightly")]
impl<In, Out> FnMut<(In,)> for Callback<In, Out> {
    extern "rust-call" fn call_mut(&mut self, args: (In,)) -> Self::Output {
        Callable::call(&*self, args.0)
    }
}

#[cfg(feature = "nightly")]
impl<In, Out> Fn<(In,)> for Callback<In, Out> {
    extern "rust-call" fn call(&self, args: (In,)) -> Self::Output {
        Callable::call(self, args.0)
    }
}

/// A callback type that implements `Copy`.
///
/// `StoredCallback<In, Out>` is an alias for `StoredValue<Callback<In, Out>>`.
///
/// # Example
/// ```
/// # use leptos::*;
/// # use leptos::{Callback, StoredCallback, Callable};
/// fn test() -> impl IntoView {
///     let callback: Callback<i32, String> =
///         Callback::new(|x: i32| x.to_string());
///     let stored_callback: StoredCallback<i32, String> =
///         store_value(callback);
///     view! {
///         <div>
///             {move || stored_callback.call(1)}
///             {move || stored_callback.call(42)}
///         </div>
///     }
/// }
/// ```
///
/// Avoid using [`StoredCallback`] as the type for a prop, as its value will be stored in
/// the scope of the parent. Instead, call [`store_value`][leptos_reactive::store_value] inside your component code.
pub type StoredCallback<In, Out> = StoredValue<Callback<In, Out>>;

#[cfg(not(feature = "nightly"))]
impl<F, In, Out> Callable<In, Out> for StoredValue<F>
where
    F: Callable<In, Out>,
{
    fn call(&self, input: In) -> Out {
        self.with_value(|cb| cb.call(input))
    }
}

/// A callback type that is `Send` and `Sync` if its input type is `Send` and `Sync`.
pub struct SyncCallback<In, Out = ()>(Arc<dyn Fn(In) -> Out>);

impl<In> fmt::Debug for SyncCallback<In> {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        fmt.write_str("SyncCallback")
    }
}

impl<In, Out> Callable<In, Out> for SyncCallback<In, Out> {
    fn call(&self, input: In) -> Out {
        (self.0)(input)
    }
}

impl<In, Out> Clone for SyncCallback<In, Out> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<In: 'static, Out: 'static> SyncCallback<In, Out> {
    /// Creates a new callback from the given function.
    pub fn new<F>(fun: F) -> Self
    where
        F: Fn(In) -> Out + 'static,
    {
        Self(Arc::new(fun))
    }
}

#[cfg(test)]
mod tests {
    use crate::callback::{Callback, SyncCallback};

    struct NoClone {}

    #[test]
    fn clone_callback() {
        let callback = Callback::new(move |_no_clone: NoClone| NoClone {});
        let _cloned = callback.clone();
    }

    #[test]
    fn clone_sync_callback() {
        let callback = SyncCallback::new(move |_no_clone: NoClone| NoClone {});
        let _cloned = callback.clone();
    }

    #[test]
    fn callback_from() {
        let _callback: Callback<(), String> = (|()| "test").into();
    }

    #[test]
    fn callback_from_html() {
        use crate::html::{AnyElement, HtmlElement};
        use leptos::*;

        let _callback: Callback<String, HtmlElement<AnyElement>> =
            (|x: String| {
                view! {
                    <h1>{x}</h1>
                }
            })
            .into();
    }
}
