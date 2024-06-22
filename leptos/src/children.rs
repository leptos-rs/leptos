use leptos_dom::Fragment;
use std::rc::Rc;

/// The most common type for the `children` property on components,
/// which can only be called once.
pub type Children = Box<dyn FnOnce() -> Fragment>;

/// A type for the `children` property on components that can be called
/// more than once.
pub type ChildrenFn = Rc<dyn Fn() -> Fragment>;

/// A type for the `children` property on components that can be called
/// more than once, but may mutate the children.
pub type ChildrenFnMut = Box<dyn FnMut() -> Fragment>;

// This is to still support components that accept `Box<dyn Fn() -> Fragment>` as a children.
type BoxedChildrenFn = Box<dyn Fn() -> Fragment>;

/// This trait can be used when constructing a component that takes children without needing
/// to know exactly what children type the component expects. This is used internally by the
/// `view!` macro implementation, and can also be used explicitly when using the builder syntax.
///
/// # Examples
///
/// ## Without ToChildren
///
/// Without [ToChildren], consumers need to explicitly provide children using the type expected
/// by the component. For example, [Provider][crate::Provider]'s children need to wrapped in
/// a [Box], while [Show][crate::Show]'s children need to be wrapped in an [Rc].
///
/// ```
/// # use leptos::{ProviderProps, ShowProps};
/// # use leptos_dom::html::p;
/// # use leptos_dom::IntoView;
/// # use leptos_macro::component;
/// # use std::rc::Rc;
/// #
/// #[component]
/// fn App() -> impl IntoView {
///     (
///         ProviderProps::builder()
///             .children(Box::new(|| p().child("Foo").into_view().into()))
///             // ...
/// #           .value("Foo")
/// #           .build(),
///         ShowProps::builder()
///             .children(Rc::new(|| p().child("Foo").into_view().into()))
///             // ...
/// #           .when(|| true)
/// #           .fallback(|| p().child("foo"))
/// #           .build(),
///     )
/// }
/// ```
///
/// ## With ToChildren
///
/// With [ToChildren], consumers don't need to know exactly which type a component uses for
/// its children.
///
/// ```
/// # use leptos::{ProviderProps, ShowProps};
/// # use leptos_dom::html::p;
/// # use leptos_dom::IntoView;
/// # use leptos_macro::component;
/// # use std::rc::Rc;
/// # use leptos::ToChildren;
/// #
/// #[component]
/// fn App() -> impl IntoView {
///     (
///         ProviderProps::builder()
///             .children(ToChildren::to_children(|| {
///                 p().child("Foo").into_view().into()
///             }))
///             // ...
/// #           .value("Foo")
/// #           .build(),
///         ShowProps::builder()
///             .children(ToChildren::to_children(|| {
///                 p().child("Foo").into_view().into()
///             }))
///             // ...
/// #           .when(|| true)
/// #           .fallback(|| p().child("foo"))
/// #           .build(),
///     )
/// }
pub trait ToChildren<F> {
    /// Convert the provided type to (generally a closure) to Self (generally a "children" type,
    /// e.g., [Children]). See the implementations to see exactly which input types are supported
    /// and which "children" type they are converted to.
    fn to_children(f: F) -> Self;
}

impl<F> ToChildren<F> for Children
where
    F: FnOnce() -> Fragment + 'static,
{
    #[inline]
    fn to_children(f: F) -> Self {
        Box::new(f)
    }
}

impl<F> ToChildren<F> for ChildrenFn
where
    F: Fn() -> Fragment + 'static,
{
    #[inline]
    fn to_children(f: F) -> Self {
        Rc::new(f)
    }
}

impl<F> ToChildren<F> for ChildrenFnMut
where
    F: FnMut() -> Fragment + 'static,
{
    #[inline]
    fn to_children(f: F) -> Self {
        Box::new(f)
    }
}

impl<F> ToChildren<F> for BoxedChildrenFn
where
    F: Fn() -> Fragment + 'static,
{
    #[inline]
    fn to_children(f: F) -> Self {
        Box::new(f)
    }
}
