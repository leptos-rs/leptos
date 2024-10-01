use crate::{
    html::{
        attribute::Attribute,
        class::IntoClass,
        event::{on, EventDescriptor},
        style::IntoStyle,
    },
    renderer::RemoveEventHandler,
};
use wasm_bindgen::JsValue;
use web_sys::Element;

/// Extends the [`Element`](Renderer::Element) type of a [`Renderer`], allowing you to add
/// attributes and children to the element's built state at runtime, with a similar API to how they
/// can be added to the static view tree at compile time.
///
/// ```rust,ignore
/// use tachys::html::element::ElementExt;
///
/// let view: HtmlElement<_, _, _, MockDom> = button();
///
/// // add an event listener as part of the static type
/// // this will be lazily added when the element is built
/// let view = element.on(ev::click, move |_| /* ... */);
///
/// // `element` now contains the actual element
/// let element = element.build();
/// let remove = element.on(ev::blur, move |_| /* ... */);
/// ```
pub trait ElementExt {
    /// Adds an attribute to the element, at runtime.
    fn attr<At>(&self, attribute: At) -> At::State
    where
        At: Attribute;

    /// Adds a class to the element, at runtime.
    fn class<C>(&self, class: C) -> C::State
    where
        C: IntoClass;

    /// Adds a style to the element, at runtime.
    fn style<S>(&self, style: S) -> S::State
    where
        S: IntoStyle;

    /// Adds an event listener to the element, at runtime.
    fn on<E>(
        &self,
        ev: E,
        cb: impl FnMut(E::EventType) + 'static,
    ) -> RemoveEventHandler<Element>
    where
        E: EventDescriptor + Send + 'static,
        E::EventType: 'static,
        E::EventType: From<JsValue>;
}

impl<T> ElementExt for T
where
    T: AsRef<Element>,
{
    fn attr<At>(&self, attribute: At) -> At::State
    where
        At: Attribute,
    {
        attribute.build(self.as_ref())
    }

    fn class<C>(&self, class: C) -> C::State
    where
        C: IntoClass,
    {
        class.build(self.as_ref())
    }

    fn on<E>(
        &self,
        ev: E,
        cb: impl FnMut(E::EventType) + 'static,
    ) -> RemoveEventHandler<Element>
    where
        E: EventDescriptor + Send + 'static,
        E::EventType: 'static,
        E::EventType: From<JsValue>,
    {
        on::<E, _>(ev, cb).attach(self.as_ref())
    }

    fn style<S>(&self, style: S) -> S::State
    where
        S: IntoStyle,
    {
        style.build(self.as_ref())
    }
}
