use crate::into_view::{IntoView, View};
use std::{
    fmt::{self, Debug},
    sync::Arc,
};
use tachys::view::{
    any_view::{AnyView, IntoAny},
    fragment::{Fragment, IntoFragment},
    RenderHtml,
};

/// The most common type for the `children` property on components,
/// which can only be called once.
///
/// This does not support iterating over individual nodes within the children.
/// To iterate over children, use [`ChildrenFragment`].
pub type Children = Box<dyn FnOnce() -> AnyView + Send>;

/// A type for the `children` property on components that can be called only once,
/// and provides a collection of all the children passed to this component.
pub type ChildrenFragment = Box<dyn FnOnce() -> Fragment + Send>;

/// A type for the `children` property on components that can be called
/// more than once.
pub type ChildrenFn = Arc<dyn Fn() -> AnyView + Send + Sync>;

/// A type for the `children` property on components that can be called more than once,
/// and provides a collection of all the children passed to this component.
pub type ChildrenFragmentFn = Arc<dyn Fn() -> Fragment + Send>;

/// A type for the `children` property on components that can be called
/// more than once, but may mutate the children.
pub type ChildrenFnMut = Box<dyn FnMut() -> AnyView + Send>;

/// A type for the `children` property on components that can be called more than once,
/// but may mutate the children, and provides a collection of all the children
/// passed to this component.
pub type ChildrenFragmentMut = Box<dyn FnMut() -> Fragment + Send>;

// This is to still support components that accept `Box<dyn Fn() -> AnyView>` as a children.
type BoxedChildrenFn = Box<dyn Fn() -> AnyView + Send>;

/// This trait can be used when constructing a component that takes children without needing
/// to know exactly what children type the component expects. This is used internally by the
/// `view!` macro implementation, and can also be used explicitly when using the builder syntax.
///
///
/// Different component types take different types for their `children` prop, some of which cannot
/// be directly constructed. Using `ToChildren` allows the component user to pass children without
/// explicity constructing the correct type.
///
/// ## Examples
///
/// ```
/// # use leptos::prelude::*;
/// # use leptos::html::p;
/// # use leptos::IntoView;
/// # use leptos_macro::component;
/// # use leptos::children::ToChildren;
/// use leptos::context::{Provider, ProviderProps};
/// use leptos::control_flow::{Show, ShowProps};
///
/// #[component]
/// fn App() -> impl IntoView {
///     (
///       Provider(
///         ProviderProps::builder()
///             .children(ToChildren::to_children(|| {
///                 p().child("Foo")
///             }))
///             // ...
///            .value("Foo")
///            .build(),
///        ),
///        Show(
///          ShowProps::builder()
///             .children(ToChildren::to_children(|| {
///                 p().child("Foo")
///             }))
///             // ...
///             .when(|| true)
///             .fallback(|| p().child("foo"))
///             .build(),
///        )
///     )
/// }
pub trait ToChildren<F> {
    /// Convert the provided type to (generally a closure) to Self (generally a "children" type,
    /// e.g., [Children]). See the implementations to see exactly which input types are supported
    /// and which "children" type they are converted to.
    fn to_children(f: F) -> Self;
}

impl<F, C> ToChildren<F> for Children
where
    F: FnOnce() -> C + Send + 'static,
    C: RenderHtml + Send + 'static,
{
    #[inline]
    fn to_children(f: F) -> Self {
        Box::new(move || f().into_any())
    }
}

impl<F, C> ToChildren<F> for ChildrenFn
where
    F: Fn() -> C + Send + Sync + 'static,
    C: RenderHtml + Send + 'static,
{
    #[inline]
    fn to_children(f: F) -> Self {
        Arc::new(move || f().into_any())
    }
}

impl<F, C> ToChildren<F> for ChildrenFnMut
where
    F: Fn() -> C + Send + 'static,
    C: RenderHtml + Send + 'static,
{
    #[inline]
    fn to_children(f: F) -> Self {
        Box::new(move || f().into_any())
    }
}

impl<F, C> ToChildren<F> for BoxedChildrenFn
where
    F: Fn() -> C + Send + 'static,
    C: RenderHtml + Send + 'static,
{
    #[inline]
    fn to_children(f: F) -> Self {
        Box::new(move || f().into_any())
    }
}

impl<F, C> ToChildren<F> for ChildrenFragment
where
    F: FnOnce() -> C + Send + 'static,
    C: IntoFragment,
{
    #[inline]
    fn to_children(f: F) -> Self {
        Box::new(move || f().into_fragment())
    }
}

impl<F, C> ToChildren<F> for ChildrenFragmentFn
where
    F: Fn() -> C + Send + 'static,
    C: IntoFragment,
{
    #[inline]
    fn to_children(f: F) -> Self {
        Arc::new(move || f().into_fragment())
    }
}

impl<F, C> ToChildren<F> for ChildrenFragmentMut
where
    F: FnMut() -> C + Send + 'static,
    C: IntoFragment,
{
    #[inline]
    fn to_children(mut f: F) -> Self {
        Box::new(move || f().into_fragment())
    }
}

/// New-type wrapper for a function that returns a view with `From` and `Default` traits implemented
/// to enable optional props in for example `<Show>` and `<Suspense>`.
#[derive(Clone)]
pub struct ViewFn(Arc<dyn Fn() -> AnyView + Send + Sync + 'static>);

impl Default for ViewFn {
    fn default() -> Self {
        Self(Arc::new(|| ().into_any()))
    }
}

impl<F, C> From<F> for ViewFn
where
    F: Fn() -> C + Send + Sync + 'static,
    C: RenderHtml + Send + 'static,
{
    fn from(value: F) -> Self {
        Self(Arc::new(move || value().into_any()))
    }
}

impl ViewFn {
    /// Execute the wrapped function
    pub fn run(&self) -> AnyView {
        (self.0)()
    }
}

/// New-type wrapper for a function, which will only be called once and returns a view with `From` and
/// `Default` traits implemented to enable optional props in for example `<Show>` and `<Suspense>`.
pub struct ViewFnOnce(Box<dyn FnOnce() -> AnyView + Send + 'static>);

impl Default for ViewFnOnce {
    fn default() -> Self {
        Self(Box::new(|| ().into_any()))
    }
}

impl<F, C> From<F> for ViewFnOnce
where
    F: FnOnce() -> C + Send + 'static,
    C: RenderHtml + Send + 'static,
{
    fn from(value: F) -> Self {
        Self(Box::new(move || value().into_any()))
    }
}

impl ViewFnOnce {
    /// Execute the wrapped function
    pub fn run(self) -> AnyView {
        (self.0)()
    }
}

/// A typed equivalent to [`Children`], which takes a generic but preserves type information to
/// allow the compiler to optimize the view more effectively.
pub struct TypedChildren<T>(Box<dyn FnOnce() -> View<T> + Send>);

impl<T> TypedChildren<T> {
    pub fn into_inner(self) -> impl FnOnce() -> View<T> + Send {
        self.0
    }
}

impl<F, C> ToChildren<F> for TypedChildren<C>
where
    F: FnOnce() -> C + Send + 'static,
    C: IntoView,
    C::AsyncOutput: Send,
{
    #[inline]
    fn to_children(f: F) -> Self {
        TypedChildren(Box::new(move || f().into_view()))
    }
}

/// A typed equivalent to [`ChildrenMut`], which takes a generic but preserves type information to
/// allow the compiler to optimize the view more effectively.
pub struct TypedChildrenMut<T>(Box<dyn FnMut() -> View<T> + Send>);

impl<T> Debug for TypedChildrenMut<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("TypedChildrenMut").finish()
    }
}

impl<T> TypedChildrenMut<T> {
    pub fn into_inner(self) -> impl FnMut() -> View<T> + Send {
        self.0
    }
}

impl<F, C> ToChildren<F> for TypedChildrenMut<C>
where
    F: FnMut() -> C + Send + 'static,
    C: IntoView,
    C::AsyncOutput: Send,
{
    #[inline]
    fn to_children(mut f: F) -> Self {
        TypedChildrenMut(Box::new(move || f().into_view()))
    }
}

/// A typed equivalent to [`ChildrenFn`], which takes a generic but preserves type information to
/// allow the compiler to optimize the view more effectively.
pub struct TypedChildrenFn<T>(Arc<dyn Fn() -> View<T> + Send + Sync>);

impl<T> Debug for TypedChildrenFn<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("TypedChildrenFn").finish()
    }
}

impl<T> TypedChildrenFn<T> {
    pub fn into_inner(self) -> Arc<dyn Fn() -> View<T> + Send + Sync> {
        self.0
    }
}

impl<F, C> ToChildren<F> for TypedChildrenFn<C>
where
    F: Fn() -> C + Send + Sync + 'static,
    C: IntoView,
    C::AsyncOutput: Send,
{
    #[inline]
    fn to_children(f: F) -> Self {
        TypedChildrenFn(Arc::new(move || f().into_view()))
    }
}
