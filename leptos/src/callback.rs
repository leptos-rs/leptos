//! Callbacks define a standard way to store functions and closures,
//! in particular for component properties.
//!
//! # How to use them
//! You can always create a callback from a closure, but the prefered way is to use `prop(into)`
//! when you define your component:
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
//!         <MyComponent render_number = |x: i32| x.to_string()/>
//!     }
//! }
//! ```
//!
//! *Notes*:
//! - in this example, you should use a generic type that implements `Fn(i32) -> String`.
//!   Callbacks are more usefull when you want optional generic props.
//! - All callbacks implement the `Callable` trait. You have to write `my_callback.call(input)`
//! - On nightly, you have to use `render_number(42)`, because callback implements Fn.
//!
//!
//! # Types
//! This modules defines:
//! - [Callback], the most basic callback type
//! - [SyncCallback] for scenarios when you need `Send` and `Sync`
//!
//! # Copying vs cloning
//! All callbacks type defined in this module are [Clone] but not [Copy].
//! To solve this issue, use [StoredValue]; see [StoredCallback] for more
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
/// # use leptos::leptos_dom::{Callable, Callback};
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
    /// creates a new callback from the function or closure
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
    T: Into<Out> + 'static
{
    fn from(f: F) -> Callback<In, Out> {
        Callback::new(move |x| f(x).into())
    }
}


// will allow to implement `Fn` for Callback in the future if needed.
#[cfg(feature = "nightly")]
auto trait NotRawCallback {}
#[cfg(feature = "nightly")]
impl<A,B> !NotRawCallback for Callback<A,B> { }
#[cfg(feature = "nightly")]
impl<F, In, T, Out> From<F> for Callback<In, Out>
where
    F: Fn(In) -> T + NotRawCallback + 'static,
    T: Into<Out> + 'static
{
    fn from(f: F) -> Callback<In, Out> {
        Callback::new(move |x| f(x).into())
    }
}


/// A callback type that implements `Copy`.
/// `StoredCallback<In,Out>` is an alias for `StoredValue<Callback<In, Out>>`.
///
/// # Example
/// ```
/// # use leptos::*;
/// # use leptos::leptos_dom::{Callback, StoredCallback, Callable};
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
/// Note that in this example, you can replace `Callback` by `SyncCallback`,
/// and it will work in the same way.
///
///
/// Note that a prop should never be a [StoredCallback]:
/// you have to call [store_value][leptos_reactive::store_value] inside your component code.
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

/// a callback type that is `Send` and `Sync` if the input type is
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
    /// creates a new callback from the function or closure
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
        let _callback : Callback<(), String> = (|()| "test").into();
    }

    #[test]
    fn callback_from_html() {
        use leptos_macro::view;
        use leptos_dom::IntoView;
        use crate::html::{HtmlElement, AnyElement};

        let _callback : Callback<String, HtmlElement<AnyElement>> 
            = (|x: String| view!{<h1>{x}</h1>}).into();
    }
}
