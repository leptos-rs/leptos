use crate::into_view::{IntoView, View};
use std::{
    fmt::{self, Debug},
    sync::Arc,
};
use tachys::{
    renderer::dom::Dom,
    view::{
        any_view::{AnyView, IntoAny},
        RenderHtml,
    },
};

/// The most common type for the `children` property on components,
/// which can only be called once.
pub type Children = Box<dyn FnOnce() -> AnyView<Dom>>;

/// A type for the `children` property on components that can be called
/// more than once.
pub type ChildrenFn = Arc<dyn Fn() -> AnyView<Dom>>;

/// A type for the `children` property on components that can be called
/// more than once, but may mutate the children.
pub type ChildrenFnMut = Box<dyn FnMut() -> AnyView<Dom>>;

// This is to still support components that accept `Box<dyn Fn() -> AnyView>` as a children.
type BoxedChildrenFn = Box<dyn Fn() -> AnyView<Dom>>;

#[doc(hidden)]
pub trait ToChildren<F> {
    fn to_children(f: F) -> Self;
}

impl<F, C> ToChildren<F> for Children
where
    F: FnOnce() -> C + Send + 'static,
    C: RenderHtml<Dom> + Send + 'static,
{
    #[inline]
    fn to_children(f: F) -> Self {
        Box::new(move || f().into_any())
    }
}

impl<F, C> ToChildren<F> for ChildrenFn
where
    F: Fn() -> C + Send + 'static,
    C: RenderHtml<Dom> + Send + 'static,
{
    #[inline]
    fn to_children(f: F) -> Self {
        Arc::new(move || f().into_any())
    }
}

impl<F, C> ToChildren<F> for ChildrenFnMut
where
    F: Fn() -> C + Send + 'static,
    C: RenderHtml<Dom> + Send + 'static,
{
    #[inline]
    fn to_children(f: F) -> Self {
        Box::new(move || f().into_any())
    }
}

impl<F, C> ToChildren<F> for BoxedChildrenFn
where
    F: Fn() -> C + 'static,
    C: RenderHtml<Dom> + Send + 'static,
{
    #[inline]
    fn to_children(f: F) -> Self {
        Box::new(move || f().into_any())
    }
}

/// New-type wrapper for the a function that returns a view with `From` and `Default` traits implemented
/// to enable optional props in for example `<Show>` and `<Suspense>`.
#[derive(Clone)]
pub struct ViewFn(Arc<dyn Fn() -> AnyView<Dom> + Send + Sync + 'static>);

impl Default for ViewFn {
    fn default() -> Self {
        Self(Arc::new(|| ().into_any()))
    }
}

impl<F, C> From<F> for ViewFn
where
    F: Fn() -> C + Send + Sync + 'static,
    C: RenderHtml<Dom> + Send + 'static,
{
    fn from(value: F) -> Self {
        Self(Arc::new(move || value().into_any()))
    }
}

impl ViewFn {
    /// Execute the wrapped function
    pub fn run(&self) -> AnyView<Dom> {
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
{
    #[inline]
    fn to_children(f: F) -> Self {
        TypedChildrenFn(Arc::new(move || f().into_view()))
    }
}
