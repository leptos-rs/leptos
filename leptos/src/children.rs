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

#[doc(hidden)]
pub trait ToChildren<F> {
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
