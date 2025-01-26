use super::attribute::{
    maybe_next_attr_erasure_macros::next_attr_output_type, Attribute,
    NextAttribute,
};
use crate::{
    html::attribute::maybe_next_attr_erasure_macros::next_attr_combine,
    renderer::Rndr,
    view::{Position, ToTemplate},
};
use std::{borrow::Cow, future::Future, sync::Arc};

/// Adds a CSS class.
#[inline(always)]
pub fn class<C>(class: C) -> Class<C>
where
    C: IntoClass,
{
    Class { class }
}

/// A CSS class.
#[derive(Debug)]
pub struct Class<C> {
    class: C,
}

impl<C> Clone for Class<C>
where
    C: Clone,
{
    fn clone(&self) -> Self {
        Self {
            class: self.class.clone(),
        }
    }
}

impl<C> Attribute for Class<C>
where
    C: IntoClass,
{
    const MIN_LENGTH: usize = C::MIN_LENGTH;

    type AsyncOutput = Class<C::AsyncOutput>;
    type State = C::State;
    type Cloneable = Class<C::Cloneable>;
    type CloneableOwned = Class<C::CloneableOwned>;

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

    fn hydrate<const FROM_SERVER: bool>(
        self,
        el: &crate::renderer::types::Element,
    ) -> Self::State {
        self.class.hydrate::<FROM_SERVER>(el)
    }

    fn build(self, el: &crate::renderer::types::Element) -> Self::State {
        self.class.build(el)
    }

    fn rebuild(self, state: &mut Self::State) {
        self.class.rebuild(state)
    }

    fn into_cloneable(self) -> Self::Cloneable {
        Class {
            class: self.class.into_cloneable(),
        }
    }

    fn into_cloneable_owned(self) -> Self::CloneableOwned {
        Class {
            class: self.class.into_cloneable_owned(),
        }
    }

    fn dry_resolve(&mut self) {
        self.class.dry_resolve();
    }

    async fn resolve(self) -> Self::AsyncOutput {
        Class {
            class: self.class.resolve().await,
        }
    }
}

impl<C> NextAttribute for Class<C>
where
    C: IntoClass,
{
    next_attr_output_type!(Self, NewAttr);

    fn add_any_attr<NewAttr: Attribute>(
        self,
        new_attr: NewAttr,
    ) -> Self::Output<NewAttr> {
        next_attr_combine!(self, new_attr)
    }
}

impl<C> ToTemplate for Class<C>
where
    C: IntoClass,
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

/// A possible value for a CSS class.
pub trait IntoClass: Send {
    /// The HTML that should be included in a `<template>`.
    const TEMPLATE: &'static str = "";
    /// The minimum length of the HTML.
    const MIN_LENGTH: usize = Self::TEMPLATE.len();

    /// The type after all async data have resolved.
    type AsyncOutput: IntoClass;
    /// The view state retained between building and rebuilding.
    type State;
    /// An equivalent value that can be cloned.
    type Cloneable: IntoClass + Clone;
    /// An equivalent value that can be cloned and is `'static`.
    type CloneableOwned: IntoClass + Clone + 'static;

    /// The estimated length of the HTML.
    fn html_len(&self) -> usize;

    /// Renders the class to HTML.
    fn to_html(self, class: &mut String);

    /// Renders the class to HTML for a `<template>`.
    #[allow(unused)] // it's used with `nightly` feature
    fn to_template(class: &mut String) {}

    /// Adds interactivity as necessary, given DOM nodes that were created from HTML that has
    /// either been rendered on the server, or cloned for a `<template>`.
    fn hydrate<const FROM_SERVER: bool>(
        self,
        el: &crate::renderer::types::Element,
    ) -> Self::State;

    /// Adds this class to the element during client-side rendering.
    fn build(self, el: &crate::renderer::types::Element) -> Self::State;

    /// Updates the value.
    fn rebuild(self, state: &mut Self::State);

    /// Converts this to a cloneable type.
    fn into_cloneable(self) -> Self::Cloneable;

    /// Converts this to a cloneable, owned type.
    fn into_cloneable_owned(self) -> Self::CloneableOwned;

    /// “Runs” the attribute without other side effects. For primitive types, this is a no-op. For
    /// reactive types, this can be used to gather data about reactivity or about asynchronous data
    /// that needs to be loaded.
    fn dry_resolve(&mut self);

    /// “Resolves” this into a type that is not waiting for any asynchronous data.
    fn resolve(self) -> impl Future<Output = Self::AsyncOutput> + Send;

    /// Reset the class list to the state before this class was added.
    fn reset(state: &mut Self::State);
}

impl<T: IntoClass> IntoClass for Option<T> {
    type AsyncOutput = Option<T::AsyncOutput>;
    type State = (crate::renderer::types::Element, Option<T::State>);
    type Cloneable = Option<T::Cloneable>;
    type CloneableOwned = Option<T::CloneableOwned>;

    fn html_len(&self) -> usize {
        self.as_ref().map_or(0, IntoClass::html_len)
    }

    fn to_html(self, class: &mut String) {
        if let Some(t) = self {
            t.to_html(class);
        }
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        el: &crate::renderer::types::Element,
    ) -> Self::State {
        if let Some(t) = self {
            (el.clone(), Some(t.hydrate::<FROM_SERVER>(el)))
        } else {
            (el.clone(), None)
        }
    }

    fn build(self, el: &crate::renderer::types::Element) -> Self::State {
        if let Some(t) = self {
            (el.clone(), Some(t.build(el)))
        } else {
            (el.clone(), None)
        }
    }

    fn rebuild(self, state: &mut Self::State) {
        let el = &state.0;
        let prev_state = &mut state.1;
        let maybe_next_t_state = match (prev_state.take(), self) {
            (Some(mut prev_t_state), None) => {
                T::reset(&mut prev_t_state);
                Some(None)
            }
            (None, Some(t)) => Some(Some(t.build(el))),
            (Some(mut prev_t_state), Some(t)) => {
                t.rebuild(&mut prev_t_state);
                Some(Some(prev_t_state))
            }
            (None, None) => Some(None),
        };
        if let Some(next_t_state) = maybe_next_t_state {
            state.1 = next_t_state;
        }
    }

    fn into_cloneable(self) -> Self::Cloneable {
        self.map(|t| t.into_cloneable())
    }

    fn into_cloneable_owned(self) -> Self::CloneableOwned {
        self.map(|t| t.into_cloneable_owned())
    }

    fn dry_resolve(&mut self) {
        if let Some(t) = self {
            t.dry_resolve();
        }
    }

    async fn resolve(self) -> Self::AsyncOutput {
        if let Some(t) = self {
            Some(t.resolve().await)
        } else {
            None
        }
    }

    fn reset(state: &mut Self::State) {
        if let Some(prev_t_state) = &mut state.1 {
            T::reset(prev_t_state);
        }
    }
}

impl IntoClass for &str {
    type AsyncOutput = Self;
    type State = (crate::renderer::types::Element, Self);
    type Cloneable = Self;
    type CloneableOwned = Arc<str>;

    fn html_len(&self) -> usize {
        self.len()
    }

    fn to_html(self, class: &mut String) {
        class.push_str(self);
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        el: &crate::renderer::types::Element,
    ) -> Self::State {
        if !FROM_SERVER {
            Rndr::set_attribute(el, "class", self);
        }
        (el.clone(), self)
    }

    fn build(self, el: &crate::renderer::types::Element) -> Self::State {
        Rndr::set_attribute(el, "class", self);
        (el.clone(), self)
    }

    fn rebuild(self, state: &mut Self::State) {
        let (el, prev) = state;
        if self != *prev {
            Rndr::set_attribute(el, "class", self);
        }
        *prev = self;
    }

    fn into_cloneable(self) -> Self::Cloneable {
        self
    }

    fn into_cloneable_owned(self) -> Self::CloneableOwned {
        self.into()
    }

    fn dry_resolve(&mut self) {}

    async fn resolve(self) -> Self::AsyncOutput {
        self
    }

    fn reset(state: &mut Self::State) {
        let (el, _prev) = state;
        Rndr::remove_attribute(el, "class");
    }
}

impl IntoClass for Cow<'_, str> {
    type AsyncOutput = Self;
    type State = (crate::renderer::types::Element, Self);
    type Cloneable = Arc<str>;
    type CloneableOwned = Arc<str>;

    fn html_len(&self) -> usize {
        self.len()
    }

    fn to_html(self, class: &mut String) {
        IntoClass::to_html(&*self, class);
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        el: &crate::renderer::types::Element,
    ) -> Self::State {
        if !FROM_SERVER {
            Rndr::set_attribute(el, "class", &self);
        }
        (el.clone(), self)
    }

    fn build(self, el: &crate::renderer::types::Element) -> Self::State {
        Rndr::set_attribute(el, "class", &self);
        (el.clone(), self)
    }

    fn rebuild(self, state: &mut Self::State) {
        let (el, prev) = state;
        if self != *prev {
            Rndr::set_attribute(el, "class", &self);
        }
        *prev = self;
    }

    fn into_cloneable(self) -> Self::Cloneable {
        self.into()
    }

    fn into_cloneable_owned(self) -> Self::CloneableOwned {
        self.into()
    }

    fn dry_resolve(&mut self) {}

    async fn resolve(self) -> Self::AsyncOutput {
        self
    }

    fn reset(state: &mut Self::State) {
        let (el, _prev) = state;
        Rndr::remove_attribute(el, "class");
    }
}

impl IntoClass for String {
    type AsyncOutput = Self;
    type State = (crate::renderer::types::Element, Self);
    type Cloneable = Arc<str>;
    type CloneableOwned = Arc<str>;

    fn html_len(&self) -> usize {
        self.len()
    }

    fn to_html(self, class: &mut String) {
        IntoClass::to_html(self.as_str(), class);
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        el: &crate::renderer::types::Element,
    ) -> Self::State {
        if !FROM_SERVER {
            Rndr::set_attribute(el, "class", &self);
        }
        (el.clone(), self)
    }

    fn build(self, el: &crate::renderer::types::Element) -> Self::State {
        Rndr::set_attribute(el, "class", &self);
        (el.clone(), self)
    }

    fn rebuild(self, state: &mut Self::State) {
        let (el, prev) = state;
        if self != *prev {
            Rndr::set_attribute(el, "class", &self);
        }
        *prev = self;
    }

    fn into_cloneable(self) -> Self::Cloneable {
        self.into()
    }

    fn into_cloneable_owned(self) -> Self::CloneableOwned {
        self.into()
    }

    fn dry_resolve(&mut self) {}

    async fn resolve(self) -> Self::AsyncOutput {
        self
    }

    fn reset(state: &mut Self::State) {
        let (el, _prev) = state;
        Rndr::remove_attribute(el, "class");
    }
}

impl IntoClass for Arc<str> {
    type AsyncOutput = Self;
    type State = (crate::renderer::types::Element, Self);
    type Cloneable = Self;
    type CloneableOwned = Self;

    fn html_len(&self) -> usize {
        self.len()
    }

    fn to_html(self, class: &mut String) {
        IntoClass::to_html(self.as_ref(), class);
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        el: &crate::renderer::types::Element,
    ) -> Self::State {
        if !FROM_SERVER {
            Rndr::set_attribute(el, "class", &self);
        }
        (el.clone(), self)
    }

    fn build(self, el: &crate::renderer::types::Element) -> Self::State {
        Rndr::set_attribute(el, "class", &self);
        (el.clone(), self)
    }

    fn rebuild(self, state: &mut Self::State) {
        let (el, prev) = state;
        if !Arc::ptr_eq(&self, prev) {
            Rndr::set_attribute(el, "class", &self);
        }
        *prev = self;
    }

    fn into_cloneable(self) -> Self::Cloneable {
        self
    }

    fn into_cloneable_owned(self) -> Self::CloneableOwned {
        self
    }

    fn dry_resolve(&mut self) {}

    async fn resolve(self) -> Self::AsyncOutput {
        self
    }

    fn reset(state: &mut Self::State) {
        let (el, _prev) = state;
        Rndr::remove_attribute(el, "class");
    }
}

impl IntoClass for (&'static str, bool) {
    type AsyncOutput = Self;
    type State = (crate::renderer::types::ClassList, bool, &'static str);
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

    fn hydrate<const FROM_SERVER: bool>(
        self,
        el: &crate::renderer::types::Element,
    ) -> Self::State {
        let (name, include) = self;
        let class_list = Rndr::class_list(el);
        if !FROM_SERVER && include {
            Rndr::add_class(&class_list, name);
        }
        (class_list, self.1, name)
    }

    fn build(self, el: &crate::renderer::types::Element) -> Self::State {
        let (name, include) = self;
        let class_list = Rndr::class_list(el);
        if include {
            Rndr::add_class(&class_list, name);
        }
        (class_list, self.1, name)
    }

    fn rebuild(self, state: &mut Self::State) {
        let (name, include) = self;
        let (class_list, prev_include, prev_name) = state;
        if include != *prev_include {
            if include {
                Rndr::add_class(class_list, name);
            } else {
                Rndr::remove_class(class_list, name);
            }
        }
        *prev_include = include;
        *prev_name = name;
    }

    fn into_cloneable(self) -> Self::Cloneable {
        self
    }

    fn into_cloneable_owned(self) -> Self::Cloneable {
        self
    }

    fn dry_resolve(&mut self) {}

    async fn resolve(self) -> Self::AsyncOutput {
        self
    }

    fn reset(state: &mut Self::State) {
        let (class_list, _, name) = state;
        Rndr::remove_class(class_list, name);
    }
}

#[cfg(feature = "nightly")]
impl<const V: &'static str> IntoClass for crate::view::static_types::Static<V> {
    const TEMPLATE: &'static str = V;

    type AsyncOutput = Self;
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

    fn hydrate<const FROM_SERVER: bool>(
        self,
        _el: &crate::renderer::types::Element,
    ) -> Self::State {
    }

    fn build(self, el: &crate::renderer::types::Element) -> Self::State {
        Rndr::set_attribute(el, "class", V);
    }

    fn rebuild(self, _state: &mut Self::State) {}

    fn into_cloneable(self) -> Self::Cloneable {
        self
    }

    fn into_cloneable_owned(self) -> Self::CloneableOwned {
        self
    }

    fn dry_resolve(&mut self) {}

    async fn resolve(self) -> Self::AsyncOutput {
        self
    }

    fn reset(_state: &mut Self::State) {}
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
