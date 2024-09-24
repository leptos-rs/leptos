use super::ElementWithChildren;
use crate::{
    html::element::{ElementType, HtmlElement},
    renderer::{dom::Dom, Renderer},
};
use std::{fmt::Debug, marker::PhantomData};

/// Creates a custom element.
#[track_caller]
pub fn custom<E>(tag: E) -> HtmlElement<Custom<E>, (), ()>
where
    E: AsRef<str>,
{
    HtmlElement {
        tag: Custom(tag),

        attributes: (),
        children: (),
    }
}

/// A custom HTML element.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Custom<E>(E);

impl<E> ElementType for Custom<E>
where
    E: AsRef<str> + Send,
{
    type Output = web_sys::HtmlElement;

    const SELF_CLOSING: bool = false;
    const ESCAPE_CHILDREN: bool = true;
    const TAG: &'static str = "";

    fn tag(&self) -> &str {
        self.0.as_ref()
    }
}

impl<E> ElementWithChildren for Custom<E> {}
