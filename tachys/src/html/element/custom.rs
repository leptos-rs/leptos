use super::ElementWithChildren;
use crate::{
    html::element::{CreateElement, ElementType, HtmlElement},
    renderer::{dom::Dom, Renderer},
};
use std::{fmt::Debug, marker::PhantomData};

/// Creates a custom element.
#[track_caller]
pub fn custom<E, Rndr>(tag: E) -> HtmlElement<Custom<E>, (), (), Rndr>
where
    E: AsRef<str>,
    Rndr: Renderer,
{
    HtmlElement {
        tag: Custom(tag),
        rndr: PhantomData,
        attributes: (),
        children: (),
        #[cfg(debug_assertions)]
        defined_at: std::panic::Location::caller(),
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

impl<E> CreateElement<Dom> for Custom<E>
where
    E: AsRef<str>,
{
    fn create_element(&self) -> <Dom as Renderer>::Element {
        use wasm_bindgen::intern;

        crate::dom::document()
            .create_element(intern(self.0.as_ref()))
            .unwrap()
    }
}
