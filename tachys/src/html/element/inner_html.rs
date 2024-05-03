use super::{ElementWithChildren, HtmlElement};
use crate::{
    html::attribute::{Attribute, NextAttribute},
    renderer::{DomRenderer, Renderer},
    view::add_attr::AddAnyAttr,
};
use std::{marker::PhantomData, sync::Arc};

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

#[derive(Debug)]
pub struct InnerHtml<T, R> {
    value: T,
    rndr: PhantomData<R>,
}

impl<T, R> Clone for InnerHtml<T, R>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone(),
            rndr: PhantomData,
        }
    }
}

impl<T, R> Attribute<R> for InnerHtml<T, R>
where
    T: InnerHtmlValue<R>,
    R: DomRenderer,
{
    const MIN_LENGTH: usize = 0;

    type State = T::State;
    type Cloneable = InnerHtml<T::Cloneable, R>;
    type CloneableOwned = InnerHtml<T::CloneableOwned, R>;

    fn html_len(&self) -> usize {
        self.value.html_len()
    }

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

    fn into_cloneable(self) -> Self::Cloneable {
        InnerHtml {
            value: self.value.into_cloneable(),
            rndr: self.rndr,
        }
    }

    fn into_cloneable_owned(self) -> Self::CloneableOwned {
        InnerHtml {
            value: self.value.into_cloneable_owned(),
            rndr: self.rndr,
        }
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

pub trait InnerHtmlValue<R: DomRenderer>: Send {
    type State;
    type Cloneable: InnerHtmlValue<R> + Clone;
    type CloneableOwned: InnerHtmlValue<R> + Clone + 'static;

    fn html_len(&self) -> usize;

    fn to_html(self, buf: &mut String);

    fn to_template(buf: &mut String);

    fn hydrate<const FROM_SERVER: bool>(self, el: &R::Element) -> Self::State;

    fn build(self, el: &R::Element) -> Self::State;

    fn rebuild(self, state: &mut Self::State);

    fn into_cloneable(self) -> Self::Cloneable;

    fn into_cloneable_owned(self) -> Self::CloneableOwned;
}

impl<R> InnerHtmlValue<R> for String
where
    R: DomRenderer,
{
    type State = (R::Element, Self);
    type Cloneable = Arc<str>;
    type CloneableOwned = Arc<str>;

    fn html_len(&self) -> usize {
        self.len()
    }

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

    fn into_cloneable(self) -> Self::Cloneable {
        self.into()
    }

    fn into_cloneable_owned(self) -> Self::Cloneable {
        self.into()
    }
}

impl<R> InnerHtmlValue<R> for Arc<str>
where
    R: DomRenderer,
{
    type State = (R::Element, Self);
    type Cloneable = Self;
    type CloneableOwned = Self;

    fn html_len(&self) -> usize {
        self.len()
    }

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

    fn into_cloneable(self) -> Self::Cloneable {
        self
    }

    fn into_cloneable_owned(self) -> Self::Cloneable {
        self
    }
}

impl<'a, R> InnerHtmlValue<R> for &'a str
where
    R: DomRenderer,
{
    type State = (R::Element, Self);
    type Cloneable = Self;
    type CloneableOwned = Arc<str>;

    fn html_len(&self) -> usize {
        self.len()
    }

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

    fn into_cloneable(self) -> Self::Cloneable {
        self
    }

    fn into_cloneable_owned(self) -> Self::CloneableOwned {
        self.into()
    }
}

impl<T, R> InnerHtmlValue<R> for Option<T>
where
    T: InnerHtmlValue<R>,
    R: DomRenderer,
{
    type State = Option<T::State>;
    type Cloneable = Option<T::Cloneable>;
    type CloneableOwned = Option<T::CloneableOwned>;

    fn html_len(&self) -> usize {
        match self {
            Some(i) => i.html_len(),
            None => 0,
        }
    }

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

    fn into_cloneable(self) -> Self::Cloneable {
        self.map(|inner| inner.into_cloneable())
    }

    fn into_cloneable_owned(self) -> Self::CloneableOwned {
        self.map(|inner| inner.into_cloneable_owned())
    }
}
