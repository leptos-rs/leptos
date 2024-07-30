use super::attribute::{Attribute, NextAttribute};
use crate::{
    renderer::DomRenderer,
    view::{Position, ToTemplate},
};
use std::{marker::PhantomData, sync::Arc};

#[inline(always)]
pub fn class<C, R>(class: C) -> Class<C, R>
where
    C: IntoClass<R>,
    R: DomRenderer,
{
    Class {
        class,
        rndr: PhantomData,
    }
}

#[derive(Debug)]
pub struct Class<C, R> {
    class: C,
    rndr: PhantomData<R>,
}

impl<C, R> Clone for Class<C, R>
where
    C: Clone,
{
    fn clone(&self) -> Self {
        Self {
            class: self.class.clone(),
            rndr: PhantomData,
        }
    }
}

impl<C, R> Attribute<R> for Class<C, R>
where
    C: IntoClass<R>,
    R: DomRenderer,
{
    const MIN_LENGTH: usize = C::MIN_LENGTH;

    type State = C::State;
    type Cloneable = Class<C::Cloneable, R>;
    type CloneableOwned = Class<C::CloneableOwned, R>;

    fn html_len(&self) -> usize {
        self.class.html_len() + 1
    }

    fn to_html(
        self,
        _buf: &mut String,
        class: &mut String,
        _style: &mut String,
        _inner_html: &mut String,
    ) {
        class.push(' ');
        self.class.to_html(class);
    }

    fn hydrate<const FROM_SERVER: bool>(self, el: &R::Element) -> Self::State {
        self.class.hydrate::<FROM_SERVER>(el)
    }

    fn build(self, el: &R::Element) -> Self::State {
        self.class.build(el)
    }

    fn rebuild(self, state: &mut Self::State) {
        self.class.rebuild(state)
    }

    fn into_cloneable(self) -> Self::Cloneable {
        Class {
            class: self.class.into_cloneable(),
            rndr: self.rndr,
        }
    }

    fn into_cloneable_owned(self) -> Self::CloneableOwned {
        Class {
            class: self.class.into_cloneable_owned(),
            rndr: self.rndr,
        }
    }
}

impl<C, R> NextAttribute<R> for Class<C, R>
where
    C: IntoClass<R>,
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

impl<C, R> ToTemplate for Class<C, R>
where
    C: IntoClass<R>,
    R: DomRenderer,
{
    const CLASS: &'static str = C::TEMPLATE;

    fn to_template(
        _buf: &mut String,
        class: &mut String,
        _style: &mut String,
        _inner_html: &mut String,
        _position: &mut Position,
    ) {
        C::to_template(class);
    }
}

pub trait IntoClass<R: DomRenderer>: Send {
    const TEMPLATE: &'static str = "";
    const MIN_LENGTH: usize = Self::TEMPLATE.len();

    type State;
    type Cloneable: IntoClass<R> + Clone;
    type CloneableOwned: IntoClass<R> + Clone + 'static;

    fn html_len(&self) -> usize;

    fn to_html(self, class: &mut String);

    #[allow(unused)] // it's used with `nightly` feature
    fn to_template(class: &mut String) {}

    fn hydrate<const FROM_SERVER: bool>(self, el: &R::Element) -> Self::State;

    fn build(self, el: &R::Element) -> Self::State;

    fn rebuild(self, state: &mut Self::State);

    fn into_cloneable(self) -> Self::Cloneable;

    fn into_cloneable_owned(self) -> Self::CloneableOwned;
}

impl<'a, R> IntoClass<R> for &'a str
where
    R: DomRenderer,
{
    type State = (R::Element, Self);
    type Cloneable = Self;
    type CloneableOwned = Arc<str>;

    fn html_len(&self) -> usize {
        self.len()
    }

    fn to_html(self, class: &mut String) {
        class.push_str(self);
    }

    fn hydrate<const FROM_SERVER: bool>(self, el: &R::Element) -> Self::State {
        if !FROM_SERVER {
            R::set_attribute(el, "class", self);
        }
        (el.clone(), self)
    }

    fn build(self, el: &R::Element) -> Self::State {
        R::set_attribute(el, "class", self);
        (el.clone(), self)
    }

    fn rebuild(self, state: &mut Self::State) {
        let (el, prev) = state;
        if self != *prev {
            R::set_attribute(el, "class", self);
        }
        *prev = self;
    }

    fn into_cloneable(self) -> Self::Cloneable {
        self
    }

    fn into_cloneable_owned(self) -> Self::CloneableOwned {
        self.into()
    }
}

impl<R> IntoClass<R> for String
where
    R: DomRenderer,
{
    type State = (R::Element, Self);
    type Cloneable = Arc<str>;
    type CloneableOwned = Arc<str>;

    fn html_len(&self) -> usize {
        self.len()
    }

    fn to_html(self, class: &mut String) {
        IntoClass::<R>::to_html(self.as_str(), class);
    }

    fn hydrate<const FROM_SERVER: bool>(self, el: &R::Element) -> Self::State {
        if !FROM_SERVER {
            R::set_attribute(el, "class", &self);
        }
        (el.clone(), self)
    }

    fn build(self, el: &R::Element) -> Self::State {
        R::set_attribute(el, "class", &self);
        (el.clone(), self)
    }

    fn rebuild(self, state: &mut Self::State) {
        let (el, prev) = state;
        if self != *prev {
            R::set_attribute(el, "class", &self);
        }
        *prev = self;
    }

    fn into_cloneable(self) -> Self::Cloneable {
        self.into()
    }

    fn into_cloneable_owned(self) -> Self::CloneableOwned {
        self.into()
    }
}

impl<R> IntoClass<R> for Arc<str>
where
    R: DomRenderer,
{
    type State = (R::Element, Self);
    type Cloneable = Self;
    type CloneableOwned = Self;

    fn html_len(&self) -> usize {
        self.len()
    }

    fn to_html(self, class: &mut String) {
        IntoClass::<R>::to_html(self.as_ref(), class);
    }

    fn hydrate<const FROM_SERVER: bool>(self, el: &R::Element) -> Self::State {
        if !FROM_SERVER {
            R::set_attribute(el, "class", &self);
        }
        (el.clone(), self)
    }

    fn build(self, el: &R::Element) -> Self::State {
        R::set_attribute(el, "class", &self);
        (el.clone(), self)
    }

    fn rebuild(self, state: &mut Self::State) {
        let (el, prev) = state;
        if !Arc::ptr_eq(&self, prev) {
            R::set_attribute(el, "class", &self);
        }
        *prev = self;
    }

    fn into_cloneable(self) -> Self::Cloneable {
        self
    }

    fn into_cloneable_owned(self) -> Self::CloneableOwned {
        self
    }
}

impl<R> IntoClass<R> for (&'static str, bool)
where
    R: DomRenderer,
{
    type State = (R::ClassList, bool);
    type Cloneable = Self;
    type CloneableOwned = Self;

    fn html_len(&self) -> usize {
        self.0.len()
    }

    fn to_html(self, class: &mut String) {
        let (name, include) = self;
        if include {
            class.push_str(name);
        }
    }

    fn hydrate<const FROM_SERVER: bool>(self, el: &R::Element) -> Self::State {
        let (name, include) = self;
        let class_list = R::class_list(el);
        if !FROM_SERVER && include {
            R::add_class(&class_list, name);
        }
        (class_list, self.1)
    }

    fn build(self, el: &R::Element) -> Self::State {
        let (name, include) = self;
        let class_list = R::class_list(el);
        if include {
            R::add_class(&class_list, name);
        }
        (class_list, self.1)
    }

    fn rebuild(self, state: &mut Self::State) {
        let (name, include) = self;
        let (class_list, prev_include) = state;
        if include != *prev_include {
            if include {
                R::add_class(class_list, name);
            } else {
                R::remove_class(class_list, name);
            }
        }
        *prev_include = include;
    }

    fn into_cloneable(self) -> Self::Cloneable {
        self
    }

    fn into_cloneable_owned(self) -> Self::Cloneable {
        self
    }
}

#[cfg(feature = "nightly")]
impl<R, const V: &'static str> IntoClass<R>
    for crate::view::static_types::Static<V>
where
    R: DomRenderer,
{
    const TEMPLATE: &'static str = V;

    type State = ();
    type Cloneable = Self;
    type CloneableOwned = Self;

    fn html_len(&self) -> usize {
        V.len()
    }

    fn to_html(self, class: &mut String) {
        class.push_str(V);
    }

    fn to_template(class: &mut String) {
        class.push_str(V);
    }

    fn hydrate<const FROM_SERVER: bool>(self, _el: &R::Element) -> Self::State {
    }

    fn build(self, el: &R::Element) -> Self::State {
        R::set_attribute(el, "class", V);
    }

    fn rebuild(self, _state: &mut Self::State) {}

    fn into_cloneable(self) -> Self::Cloneable {
        self
    }

    fn into_cloneable_owned(self) -> Self::CloneableOwned {
        self
    }
}

/* #[cfg(test)]
mod tests {
    use crate::{
        html::{
            class::class,
            element::{p, HtmlElement},
        },
        renderer::dom::Dom,
        view::{Position, PositionState, RenderHtml},
    };

    #[test]
    fn adds_simple_class() {
        let mut html = String::new();
        let el: HtmlElement<_, _, _, Dom> = p(class("foo bar"), ());
        el.to_html(&mut html, &PositionState::new(Position::FirstChild));

        assert_eq!(html, r#"<p class="foo bar"></p>"#);
    }

    #[test]
    fn adds_class_with_dynamic() {
        let mut html = String::new();
        let el: HtmlElement<_, _, _, Dom> =
            p((class("foo bar"), class(("baz", true))), ());
        el.to_html(&mut html, &PositionState::new(Position::FirstChild));

        assert_eq!(html, r#"<p class="foo bar baz"></p>"#);
    }

    #[test]
    fn adds_class_with_dynamic_and_function() {
        let mut html = String::new();
        let el: HtmlElement<_, _, _, Dom> = p(
            (
                class("foo bar"),
                class(("baz", || true)),
                class(("boo", false)),
            ),
            (),
        );
        el.to_html(&mut html, &PositionState::new(Position::FirstChild));

        assert_eq!(html, r#"<p class="foo bar baz"></p>"#);
    }
} */
