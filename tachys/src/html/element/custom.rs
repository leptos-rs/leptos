use super::ElementWithChildren;
use crate::html::element::{ElementType, HtmlElement};
use std::fmt::Debug;

/// Creates a custom element.
#[track_caller]
pub fn custom<E>(tag: E) -> HtmlElement<Custom<E>, (), ()>
where
    E: AsRef<str>,
{
    HtmlElement {
        #[cfg(any(debug_assertions, leptos_debuginfo))]
        defined_at: std::panic::Location::caller(),
        tag: Custom(tag),
        attributes: (),
        children: (),
    }
}

/// A custom HTML element.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Custom<E>(E);

impl<E: 'static> ElementType for Custom<E>
where
    E: AsRef<str> + Send,
{
    type Output = web_sys::HtmlElement;

    const SELF_CLOSING: bool = false;
    const ESCAPE_CHILDREN: bool = true;
    const TAG: &'static str = "";
    const NAMESPACE: Option<&'static str> = None;

    fn tag(&self) -> &str {
        self.0.as_ref()
    }
}

impl<E> ElementWithChildren for Custom<E> {}
