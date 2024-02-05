use super::ElementWithChildren;
use crate::{
    html::element::{CreateElement, ElementType, HtmlElement},
    renderer::{dom::Dom, Renderer},
};
use std::{borrow::Cow, fmt::Debug, marker::PhantomData, rc::Rc, sync::Arc};

// FIXME custom element HTML rendering is broken because tag names aren't static
pub fn custom<E, Rndr>(tag: E) -> HtmlElement<Custom<E>, (), (), Rndr>
where
    E: CustomElementKey,
    Rndr: Renderer,
{
    HtmlElement {
        tag: Custom(tag),
        rndr: PhantomData,
        attributes: (),
        children: (),
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Custom<E>(E)
where
    E: CustomElementKey;

impl<E> ElementType for Custom<E>
where
    E: CustomElementKey,
{
    type Output = web_sys::HtmlElement;

    const TAG: &'static str = E::KEY;
    const SELF_CLOSING: bool = false;

    fn tag(&self) -> &str {
        self.0.as_ref()
    }
}

impl<E> ElementWithChildren for Custom<E> where E: CustomElementKey {}

impl<E> CreateElement<Dom> for Custom<E>
where
    E: CustomElementKey,
{
    fn create_element(&self) -> <Dom as Renderer>::Element {
        use wasm_bindgen::intern;

        crate::dom::document()
            .create_element(intern(self.0.as_ref()))
            .unwrap()
    }
}

// TODO these are all broken for custom elements
pub trait CustomElementKey: AsRef<str> {
    const KEY: &'static str;
}

impl<'a> CustomElementKey for &'a str {
    const KEY: &'static str = "";
}

impl<'a> CustomElementKey for Cow<'a, str> {
    const KEY: &'static str = "";
}

impl CustomElementKey for &String {
    const KEY: &'static str = "";
}

impl CustomElementKey for String {
    const KEY: &'static str = "";
}

impl CustomElementKey for Rc<str> {
    const KEY: &'static str = "";
}

impl CustomElementKey for Arc<str> {
    const KEY: &'static str = "";
}

#[cfg(feature = "nightly")]
impl<const K: &'static str> CustomElementKey
    for crate::view::static_types::Static<K>
{
    const KEY: &'static str = K;
}
