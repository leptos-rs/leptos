use super::{ElementWithChildren, HtmlElement};
use crate::{
    html::attribute::{Attribute, NextAttribute},
    renderer::{DomRenderer, Renderer},
    view::add_attr::AddAnyAttr,
};
use std::{marker::PhantomData, rc::Rc, sync::Arc};

#[inline(always)]
pub fn inner_html<T, R>(value: T) -> InnerHtml<T, R>
where
    T: InnerHtmlValue<R>,
    R: DomRenderer,
{
    InnerHtml {
        value,
        rndr: PhantomData,
    }
}

#[derive(Debug, Clone, Copy)]
pub struct InnerHtml<T, R>
where
    T: InnerHtmlValue<R>,
    R: DomRenderer,
{
    value: T,
    rndr: PhantomData<R>,
}

impl<T, R> Attribute<R> for InnerHtml<T, R>
where
    T: InnerHtmlValue<R>,
    R: DomRenderer,
{
    const MIN_LENGTH: usize = 0;

    type State = T::State;

    fn to_html(
        self,
        _buf: &mut String,
        _class: &mut String,
        _style: &mut String,
        inner_html: &mut String,
    ) {
        self.value.to_html(inner_html);
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        el: &<R as Renderer>::Element,
    ) -> Self::State {
        self.value.hydrate::<FROM_SERVER>(el)
    }

    fn build(self, el: &<R as Renderer>::Element) -> Self::State {
        self.value.build(el)
    }

    fn rebuild(self, state: &mut Self::State) {
        self.value.rebuild(state);
    }
}

impl<T, R> NextAttribute<R> for InnerHtml<T, R>
where
    T: InnerHtmlValue<R>,
    R: DomRenderer,
{
    type Output<NewAttr: Attribute<R>> = (Self, NewAttr);

    fn add_any_attr<NewAttr: Attribute<R>>(
        self,
        new_attr: NewAttr,
    ) -> Self::Output<NewAttr> {
        (self, new_attr)
    }
}

pub trait InnerHtmlAttribute<T, Rndr>
where
    T: InnerHtmlValue<Rndr>,
    Rndr: DomRenderer,
    Self: Sized + AddAnyAttr<Rndr>,
{
    fn inner_html(
        self,
        value: T,
    ) -> <Self as AddAnyAttr<Rndr>>::Output<InnerHtml<T, Rndr>> {
        self.add_any_attr(inner_html(value))
    }
}

impl<T, E, At, Rndr> InnerHtmlAttribute<T, Rndr>
    for HtmlElement<E, At, (), Rndr>
where
    Self: AddAnyAttr<Rndr>,
    E: ElementWithChildren,
    At: Attribute<Rndr>,
    T: InnerHtmlValue<Rndr>,
    Rndr: DomRenderer,
{
    fn inner_html(
        self,
        value: T,
    ) -> <Self as AddAnyAttr<Rndr>>::Output<InnerHtml<T, Rndr>> {
        self.add_any_attr(inner_html(value))
    }
}

pub trait InnerHtmlValue<R: DomRenderer> {
    type State;

    fn to_html(self, buf: &mut String);

    fn to_template(buf: &mut String);

    fn hydrate<const FROM_SERVER: bool>(self, el: &R::Element) -> Self::State;

    fn build(self, el: &R::Element) -> Self::State;

    fn rebuild(self, state: &mut Self::State);
}

impl<R> InnerHtmlValue<R> for String
where
    R: DomRenderer,
{
    type State = (R::Element, Self);

    fn to_html(self, buf: &mut String) {
        buf.push_str(&self);
    }

    fn to_template(_buf: &mut String) {}

    fn hydrate<const FROM_SERVER: bool>(
        self,
        el: &<R as Renderer>::Element,
    ) -> Self::State {
        if !FROM_SERVER {
            R::set_inner_html(el, &self);
        }
        (el.clone(), self)
    }

    fn build(self, el: &<R as Renderer>::Element) -> Self::State {
        R::set_inner_html(el, &self);
        (el.clone(), self)
    }

    fn rebuild(self, state: &mut Self::State) {
        if self != state.1 {
            R::set_inner_html(&state.0, &self);
            state.1 = self;
        }
    }
}

impl<R> InnerHtmlValue<R> for Rc<str>
where
    R: DomRenderer,
{
    type State = (R::Element, Self);

    fn to_html(self, buf: &mut String) {
        buf.push_str(&self);
    }

    fn to_template(_buf: &mut String) {}

    fn hydrate<const FROM_SERVER: bool>(
        self,
        el: &<R as Renderer>::Element,
    ) -> Self::State {
        if !FROM_SERVER {
            R::set_inner_html(el, &self);
        }
        (el.clone(), self)
    }

    fn build(self, el: &<R as Renderer>::Element) -> Self::State {
        R::set_inner_html(el, &self);
        (el.clone(), self)
    }

    fn rebuild(self, state: &mut Self::State) {
        if !Rc::ptr_eq(&self, &state.1) {
            R::set_inner_html(&state.0, &self);
            state.1 = self;
        }
    }
}

impl<R> InnerHtmlValue<R> for Arc<str>
where
    R: DomRenderer,
{
    type State = (R::Element, Self);

    fn to_html(self, buf: &mut String) {
        buf.push_str(&self);
    }

    fn to_template(_buf: &mut String) {}

    fn hydrate<const FROM_SERVER: bool>(
        self,
        el: &<R as Renderer>::Element,
    ) -> Self::State {
        if !FROM_SERVER {
            R::set_inner_html(el, &self);
        }
        (el.clone(), self)
    }

    fn build(self, el: &<R as Renderer>::Element) -> Self::State {
        R::set_inner_html(el, &self);
        (el.clone(), self)
    }

    fn rebuild(self, state: &mut Self::State) {
        if !Arc::ptr_eq(&self, &state.1) {
            R::set_inner_html(&state.0, &self);
            state.1 = self;
        }
    }
}

impl<'a, R> InnerHtmlValue<R> for &'a str
where
    R: DomRenderer,
{
    type State = (R::Element, Self);

    fn to_html(self, buf: &mut String) {
        buf.push_str(self);
    }

    fn to_template(_buf: &mut String) {}

    fn hydrate<const FROM_SERVER: bool>(
        self,
        el: &<R as Renderer>::Element,
    ) -> Self::State {
        if !FROM_SERVER {
            R::set_inner_html(el, self);
        }
        (el.clone(), self)
    }

    fn build(self, el: &<R as Renderer>::Element) -> Self::State {
        R::set_inner_html(el, self);
        (el.clone(), self)
    }

    fn rebuild(self, state: &mut Self::State) {
        if self != state.1 {
            R::set_inner_html(&state.0, self);
            state.1 = self;
        }
    }
}

impl<T, R> InnerHtmlValue<R> for Option<T>
where
    T: InnerHtmlValue<R>,
    R: DomRenderer,
{
    type State = Option<T::State>;

    fn to_html(self, buf: &mut String) {
        if let Some(value) = self {
            value.to_html(buf);
        }
    }

    fn to_template(_buf: &mut String) {}

    fn hydrate<const FROM_SERVER: bool>(
        self,
        el: &<R as Renderer>::Element,
    ) -> Self::State {
        self.map(|n| n.hydrate::<FROM_SERVER>(el))
    }

    fn build(self, el: &<R as Renderer>::Element) -> Self::State {
        self.map(|n| n.build(el))
    }

    fn rebuild(self, state: &mut Self::State) {
        todo!()
    }
}
