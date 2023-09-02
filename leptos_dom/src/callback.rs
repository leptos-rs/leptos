//! Callbacks define a standard way to store functions and closures,
//! in particular for component properties.
//!
//! # How to use them
//! You can always create a callback from a closure, but the prefered way is to use `prop(into)`
//! when you define your component:
//! ```
//! # use leptos::*;
//! # use leptos::leptos_dom::{Callback, Callable};
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
//!
//!
//! # Types
//! This modules defines:
//! - [Callback], the most basic callback type
//! - [SyncCallback] for scenarios when you need `Send` and `Sync`
//! - [HtmlCallback] for a function that returns a [HtmlElement]
//! - [ViewCallback] for a function that returns some kind of [view][IntoView]
//!
//! # Copying vs cloning
//! All callbacks type defined in this module are [Clone] but not [Copy].
//! To solve this issue, use [StoredValue]; see [StoredCallback] for more
//! ```
//! # use leptos::*;
//! # use leptos::leptos_dom::{Callback, Callable};
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

use crate::{AnyElement, ElementDescriptor, HtmlElement, IntoView, View};
use leptos_reactive::StoredValue;
use std::{rc::Rc, sync::Arc};

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

#[derive(Clone)]
pub struct Callback<In, Out = ()>(Rc<dyn Fn(In) -> Out>);

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

impl<F, In, Out> From<F> for Callback<In, Out>
where
    F: Fn(In) -> Out + 'static,
{
    fn from(f: F) -> Callback<In, Out> {
        Callback::new(f)
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
/// Note that in this example, you can replace `Callback` by `SyncCallback` or `ViewCallback`, and
/// it will work in the same way.
///
///
/// Note that a prop should never be a [StoredCallback]:
/// you have to call [store_value][leptos_reactive::store_value] inside your component code.
pub type StoredCallback<In, Out> = StoredValue<Callback<In, Out>>;

impl<F, In, Out> Callable<In, Out> for StoredValue<F>
where
    F: Callable<In, Out>,
{
    fn call(&self, input: In) -> Out {
        self.with_value(|cb| cb.call(input))
    }
}

/// a callback type that is `Send` and `Sync` if the input type is
#[derive(Clone)]
pub struct SyncCallback<In, Out = ()>(Arc<dyn Fn(In) -> Out>);

impl<In: 'static, Out: 'static> SyncCallback<In, Out> {
    /// creates a new callback from the function or closure
    pub fn new<F>(fun: F) -> Self
    where
        F: Fn(In) -> Out + 'static,
    {
        Self(Arc::new(fun))
    }
}

/// A special callback type that returns any Html element.
/// You can use it exactly the same way as a classic callback.
///
/// For how to use callbacks, see [here][crate::callback]
///
/// # Example
///
/// ```
/// # use leptos::*;
/// # use leptos::leptos_dom::{Callable, HtmlCallback};
/// #[component]
/// fn MyComponent(
///     #[prop(into)] render_number: HtmlCallback<i32>,
/// ) -> impl IntoView {
///     view! {
///         <div>
///             {render_number.call(42)}
///         </div>
///     }
/// }
/// fn test() -> impl IntoView {
///     view! {
///         <MyComponent render_number=move |x: i32| view!{<span>{x}</span>}/>
///     }
/// }
/// ```
///
/// # `HtmlCallback` with empty input type.
/// Note that when `my_html_callback` is `HtmlCallback<()>`, you can use it more easily because it
/// implements [IntoView]
///
/// view!{
///     <div>
///         {render_number}
///     </div>
/// }
#[derive(Clone)]
pub struct HtmlCallback<In = ()>(Rc<dyn Fn(In) -> HtmlElement<AnyElement>>);

impl<In> HtmlCallback<In> {
    /// creates a new callback from the function or closure
    pub fn new<F, H>(f: F) -> Self
    where
        F: Fn(In) -> HtmlElement<H> + 'static,
        H: ElementDescriptor + 'static,
    {
        Self(Rc::new(move |x| f(x).into_any()))
    }
}

impl<In> Callable<In, HtmlElement<AnyElement>> for HtmlCallback<In> {
    fn call(&self, input: In) -> HtmlElement<AnyElement> {
        (self.0)(input)
    }
}

impl<In, F, H> From<F> for HtmlCallback<In>
where
    F: Fn(In) -> HtmlElement<H> + 'static,
    H: ElementDescriptor + 'static,
{
    fn from(f: F) -> Self {
        HtmlCallback(Rc::new(move |x| f(x).into_any()))
    }
}

impl IntoView for HtmlCallback<()> {
    fn into_view(self) -> View {
        self.call(()).into_view()
    }
}

/// A special callback type that returns any [`View`].
///
/// You can use it exactly the same way as a classic callback.
/// For how to use callbacks, see [here][crate::callback]
///
/// ```
/// # use leptos::*;
/// # use leptos::leptos_dom::{ViewCallback, Callable};
/// #[component]
/// fn MyComponent(
///     #[prop(into)] render_number: ViewCallback<i32>,
/// ) -> impl IntoView {
///     view! {
///         <div>
///             {render_number.call(42)}
///         </div>
///     }
/// }
/// fn test() -> impl IntoView {
///     view! {
///         <MyComponent render_number=move |x: i32| view!{<span>{x}</span>}/>
///     }
/// }
/// ```
///
/// # `ViewCallback` with empty input type.
/// Note that when `my_view_callback` is `ViewCallback<()>`, you can use it more easily because it
/// implements [IntoView]
///
/// view!{
///     <div>
///         {render_number}
///     </div>
/// }
#[derive(Clone)]
pub struct ViewCallback<In>(Rc<dyn Fn(In) -> View>);

impl<In> ViewCallback<In> {
    /// creates a new callback from the function or closure
    fn new<F, V>(f: F) -> Self
    where
        F: Fn(In) -> V + 'static,
        V: IntoView + 'static,
    {
        ViewCallback(Rc::new(move |x| f(x).into_view()))
    }
}

impl<In> Callable<In, View> for ViewCallback<In> {
    fn call(&self, input: In) -> View {
        (self.0)(input)
    }
}

impl<In, F, V> From<F> for ViewCallback<In>
where
    F: Fn(In) -> V + 'static,
    V: IntoView + 'static,
{
    fn from(f: F) -> Self {
        Self::new(f)
    }
}

impl IntoView for ViewCallback<()> {
    fn into_view(self) -> View {
        self.call(()).into_view()
    }
}
