// inspired by:
// https://github.com/lpotthast/leptonic/blob/f8a270ae5512561a92f5deca057f7999940de11b/leptonic/src/callback.rs

use crate::{AnyElement, ElementDescriptor, HtmlElement, IntoView, View};
use leptos_reactive::StoredValue;
use std::{rc::Rc, sync::Arc};

/// A wrapper trait for calling callbacks.
pub trait Callable<In, Out = ()> {
    /// calls the callback with the specified argument.
    fn call(&self, input: In) -> Out;
}

/// The most basic leptos callback type.
/// It is intended to make passing a function to your component easy.
///
/// # Usage when you write a component
/// ```
/// #[component]
/// fn MyComponent(#[prop(into)] render_number: Callback<i32, String>) {
///     view! {
///         <div>
///             {render_number.call(42)}
///         </div>
///     }
/// }
/// ```
/// # Usage when you use your component
/// ```
/// view! {
///     <MyComponent callback=move |x| x.to_string()/>
/// }
/// ```
///
/// Note that in this scenario, you should use a generic component instead.
/// You should use callback mainly for optional generic props.
///
/// # Cloning
/// The default [Callback] type can be cloned cheaply but cannot be copied.
/// If you want to use the callback in a lot of closures, you will have to clone it each time.
/// To avoid that, see [StoredCallback]
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

/// a callback type that can be copied.
///
/// To create it, just use [store_value] on your callback.
///
/// Note that a prop should never be a [StoredCallback]:
/// you have to call [store_value] inside your component code.
///
/// You can store any callback type, and use it as a callback.
pub type StoredCallback<In, Out> = StoredValue<Callback<In, Out>>;

impl<F, In, Out> Callable<In, Out> for StoredValue<F>
where
    F: Callable<In, Out>,
{
    fn call(&self, input: In) -> Out {
        self.with_value(|cb| cb.call(input))
    }
}

/// a callback type that
/// - can be cloned
/// - is `Send` and `Sync` if the input type is
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
///
/// # Usage when you write your component
/// You can use it exactly the same way as a classic callback.
///
///
/// ```
/// #[component]
/// fn MyComponent(#[prop(into)] render_number: HtmlCallback<i32>) {
///     view! {
///         <div>
///             {render_number.call(42)}
///         </div>
///     }
/// }
/// ```
///
/// # Usage when you use your component
/// Again, use it the same way as a classic callback.
///
/// ```
/// view! {
///     <MyComponent callback=move |x| view!{<span>{x}</span>}/>
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
pub struct HtmlCallback<In>(Rc<dyn Fn(In) -> HtmlElement<AnyElement>>);

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

/// A special callback type that returns any View
///
/// # Usage when you write your component
/// You can use it exactly the same way as a classic callback.
///
///
/// ```
/// #[component]
/// fn MyComponent(#[prop(into)] render_number: ViewCallback<i32>) {
///     view! {
///         <div>
///             {render_number.call(42)}
///         </div>
///     }
/// }
/// ```
///
/// # Usage when you use your component
/// Again, use it the same way as a classic callback.
///
/// ```
/// view! {
///     <MyComponent callback=move |x| view!{<span>{x}</span>}/>
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
