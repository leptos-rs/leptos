use super::attribute::{Attribute, NextAttribute};
#[cfg(feature = "nightly")]
use crate::view::static_types::Static;
use crate::{
    renderer::Rndr,
    view::{Position, ToTemplate},
};
use std::{future::Future, sync::Arc};

/// Returns an [`Attribute`] that will add to an element's CSS styles.
#[inline(always)]
pub fn style<S>(style: S) -> Style<S>
where
    S: IntoStyle,
{
    Style { style }
}

/// An [`Attribute`] that will add to an element's CSS styles.
#[derive(Debug)]
pub struct Style<S> {
    style: S,
}

impl<S> Clone for Style<S>
where
    S: Clone,
{
    fn clone(&self) -> Self {
        Self {
            style: self.style.clone(),
        }
    }
}

impl<S> Attribute for Style<S>
where
    S: IntoStyle,
{
    const MIN_LENGTH: usize = 0;

    type AsyncOutput = Style<S::AsyncOutput>;
    type State = S::State;
    type Cloneable = Style<S::Cloneable>;
    type CloneableOwned = Style<S::CloneableOwned>;

    // TODO
    #[inline(always)]
    fn html_len(&self) -> usize {
        0
    }

    fn to_html(
        self,
        _buf: &mut String,
        _style: &mut String,
        style: &mut String,
        _inner_html: &mut String,
    ) {
        self.style.to_html(style);
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        el: &crate::renderer::types::Element,
    ) -> Self::State {
        self.style.hydrate::<FROM_SERVER>(el)
    }

    fn build(self, el: &crate::renderer::types::Element) -> Self::State {
        self.style.build(el)
    }

    fn rebuild(self, state: &mut Self::State) {
        self.style.rebuild(state)
    }

    fn into_cloneable(self) -> Self::Cloneable {
        Style {
            style: self.style.into_cloneable(),
        }
    }

    fn into_cloneable_owned(self) -> Self::CloneableOwned {
        Style {
            style: self.style.into_cloneable_owned(),
        }
    }

    fn dry_resolve(&mut self) {
        self.style.dry_resolve();
    }

    async fn resolve(self) -> Self::AsyncOutput {
        Style {
            style: self.style.resolve().await,
        }
    }
}

impl<S> NextAttribute for Style<S>
where
    S: IntoStyle,
{
    type Output<NewAttr: Attribute> = (Self, NewAttr);

    fn add_any_attr<NewAttr: Attribute>(
        self,
        new_attr: NewAttr,
    ) -> Self::Output<NewAttr> {
        (self, new_attr)
    }
}

impl<S> ToTemplate for Style<S>
where
    S: IntoStyle,
{
    fn to_template(
        _buf: &mut String,
        _style: &mut String,
        _class: &mut String,
        _inner_html: &mut String,
        _position: &mut Position,
    ) {
        // TODO: should there be some templating for static styles?
    }
}

/// Any type that can be added to the `style` attribute or set as a style in
/// the [`CssStyleDeclaration`]. This could be a plain string, or a property name-value pair.
pub trait IntoStyle: Send {
    /// The type after all async data have resolved.
    type AsyncOutput: IntoStyle;
    /// The view state retained between building and rebuilding.
    type State;
    /// An equivalent value that can be cloned.
    type Cloneable: IntoStyle + Clone;
    /// An equivalent value that can be cloned and is `'static`.
    type CloneableOwned: IntoStyle + Clone + 'static;

    /// Renders the style to HTML.
    fn to_html(self, style: &mut String);

    /// Adds interactivity as necessary, given DOM nodes that were created from HTML that has
    /// either been rendered on the server, or cloned for a `<template>`.
    fn hydrate<const FROM_SERVER: bool>(
        self,
        el: &crate::renderer::types::Element,
    ) -> Self::State;

    /// Adds this style to the element during client-side rendering.
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
}

impl<'a> IntoStyle for &'a str {
    type AsyncOutput = Self;
    type State = (crate::renderer::types::Element, &'a str);
    type Cloneable = Self;
    type CloneableOwned = Arc<str>;

    fn to_html(self, style: &mut String) {
        style.push_str(self);
        style.push(';');
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        el: &crate::renderer::types::Element,
    ) -> Self::State {
        (el.clone(), self)
    }

    fn build(self, el: &crate::renderer::types::Element) -> Self::State {
        Rndr::set_attribute(el, "style", self);
        (el.clone(), self)
    }

    fn rebuild(self, state: &mut Self::State) {
        let (el, prev) = state;
        if self != *prev {
            Rndr::set_attribute(el, "style", self);
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
}

impl IntoStyle for Arc<str> {
    type AsyncOutput = Self;
    type State = (crate::renderer::types::Element, Arc<str>);
    type Cloneable = Self;
    type CloneableOwned = Self;

    fn to_html(self, style: &mut String) {
        style.push_str(&self);
        style.push(';');
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        el: &crate::renderer::types::Element,
    ) -> Self::State {
        (el.clone(), self)
    }

    fn build(self, el: &crate::renderer::types::Element) -> Self::State {
        Rndr::set_attribute(el, "style", &self);
        (el.clone(), self)
    }

    fn rebuild(self, state: &mut Self::State) {
        let (el, prev) = state;
        if self != *prev {
            Rndr::set_attribute(el, "style", &self);
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
}

impl IntoStyle for String {
    type AsyncOutput = Self;
    type State = (crate::renderer::types::Element, String);
    type Cloneable = Arc<str>;
    type CloneableOwned = Arc<str>;

    fn to_html(self, style: &mut String) {
        style.push_str(&self);
        style.push(';');
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        el: &crate::renderer::types::Element,
    ) -> Self::State {
        (el.clone(), self)
    }

    fn build(self, el: &crate::renderer::types::Element) -> Self::State {
        Rndr::set_attribute(el, "style", &self);
        (el.clone(), self)
    }

    fn rebuild(self, state: &mut Self::State) {
        let (el, prev) = state;
        if self != *prev {
            Rndr::set_attribute(el, "style", &self);
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
}

impl IntoStyle for (Arc<str>, Arc<str>) {
    type AsyncOutput = Self;
    type State = (crate::renderer::types::CssStyleDeclaration, Arc<str>);
    type Cloneable = Self;
    type CloneableOwned = Self;

    fn to_html(self, style: &mut String) {
        let (name, value) = self;
        style.push_str(&name);
        style.push(':');
        style.push_str(&value);
        style.push(';');
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        el: &crate::renderer::types::Element,
    ) -> Self::State {
        let style = Rndr::style(el);
        (style, self.1)
    }

    fn build(self, el: &crate::renderer::types::Element) -> Self::State {
        let (name, value) = self;
        let style = Rndr::style(el);
        Rndr::set_css_property(&style, &name, &value);
        (style, value)
    }

    fn rebuild(self, state: &mut Self::State) {
        let (name, value) = self;
        let (style, prev) = state;
        if value != *prev {
            Rndr::set_css_property(style, &name, &value);
        }
        *prev = value;
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
}

impl<'a> IntoStyle for (&'a str, &'a str) {
    type AsyncOutput = Self;
    type State = (crate::renderer::types::CssStyleDeclaration, &'a str);
    type Cloneable = Self;
    type CloneableOwned = (Arc<str>, Arc<str>);

    fn to_html(self, style: &mut String) {
        let (name, value) = self;
        style.push_str(name);
        style.push(':');
        style.push_str(value);
        style.push(';');
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        el: &crate::renderer::types::Element,
    ) -> Self::State {
        let style = Rndr::style(el);
        (style, self.1)
    }

    fn build(self, el: &crate::renderer::types::Element) -> Self::State {
        let (name, value) = self;
        let style = Rndr::style(el);
        Rndr::set_css_property(&style, name, value);
        (style, self.1)
    }

    fn rebuild(self, state: &mut Self::State) {
        let (name, value) = self;
        let (style, prev) = state;
        if value != *prev {
            Rndr::set_css_property(style, name, value);
        }
        *prev = value;
    }

    fn into_cloneable(self) -> Self::Cloneable {
        self
    }

    fn into_cloneable_owned(self) -> Self::CloneableOwned {
        (self.0.into(), self.1.into())
    }

    fn dry_resolve(&mut self) {}

    async fn resolve(self) -> Self::AsyncOutput {
        self
    }
}

impl<'a> IntoStyle for (&'a str, String) {
    type AsyncOutput = Self;
    type State = (crate::renderer::types::CssStyleDeclaration, String);
    type Cloneable = (Arc<str>, Arc<str>);
    type CloneableOwned = (Arc<str>, Arc<str>);

    fn to_html(self, style: &mut String) {
        let (name, value) = self;
        style.push_str(name);
        style.push(':');
        style.push_str(&value);
        style.push(';');
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        el: &crate::renderer::types::Element,
    ) -> Self::State {
        let style = Rndr::style(el);
        (style, self.1)
    }

    fn build(self, el: &crate::renderer::types::Element) -> Self::State {
        let (name, value) = &self;
        let style = Rndr::style(el);
        Rndr::set_css_property(&style, name, value);
        (style, self.1)
    }

    fn rebuild(self, state: &mut Self::State) {
        let (name, value) = self;
        let (style, prev) = state;
        if value != *prev {
            Rndr::set_css_property(style, name, &value);
        }
        *prev = value;
    }

    fn into_cloneable(self) -> Self::Cloneable {
        (self.0.into(), self.1.into())
    }

    fn into_cloneable_owned(self) -> Self::CloneableOwned {
        (self.0.into(), self.1.into())
    }

    fn dry_resolve(&mut self) {}

    async fn resolve(self) -> Self::AsyncOutput {
        self
    }
}

#[cfg(feature = "nightly")]
impl<'a, const V: &'static str> IntoStyle for (&'a str, Static<V>) {
    type AsyncOutput = Self;
    type State = ();
    type Cloneable = (Arc<str>, Static<V>);
    type CloneableOwned = (Arc<str>, Static<V>);

    fn to_html(self, style: &mut String) {
        let (name, _) = self;
        style.push_str(name);
        style.push(':');
        style.push_str(V);
        style.push(';');
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        _el: &crate::renderer::types::Element,
    ) -> Self::State {
    }

    fn build(self, el: &crate::renderer::types::Element) -> Self::State {
        let (name, _) = &self;
        let style = Rndr::style(el);
        Rndr::set_css_property(&style, name, V);
    }

    fn rebuild(self, _state: &mut Self::State) {}

    fn into_cloneable(self) -> Self::Cloneable {
        (self.0.into(), self.1)
    }

    fn into_cloneable_owned(self) -> Self::CloneableOwned {
        (self.0.into(), self.1)
    }

    fn dry_resolve(&mut self) {}

    async fn resolve(self) -> Self::AsyncOutput {
        self
    }
}

#[cfg(feature = "nightly")]
impl<const V: &'static str> IntoStyle for (Arc<str>, Static<V>) {
    type AsyncOutput = Self;
    type State = ();
    type Cloneable = (Arc<str>, Static<V>);
    type CloneableOwned = (Arc<str>, Static<V>);

    fn to_html(self, style: &mut String) {
        let (name, _) = self;
        style.push_str(&name);
        style.push(':');
        style.push_str(V);
        style.push(';');
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        _el: &crate::renderer::types::Element,
    ) -> Self::State {
    }

    fn build(self, el: &crate::renderer::types::Element) -> Self::State {
        let (name, _) = &self;
        let style = Rndr::style(el);
        Rndr::set_css_property(&style, name, V);
    }

    fn rebuild(self, _state: &mut Self::State) {}

    fn into_cloneable(self) -> Self::Cloneable {
        (self.0, self.1)
    }

    fn into_cloneable_owned(self) -> Self::CloneableOwned {
        (self.0, self.1)
    }

    fn dry_resolve(&mut self) {}

    async fn resolve(self) -> Self::AsyncOutput {
        self
    }
}

#[cfg(feature = "nightly")]
impl<const V: &'static str> IntoStyle for crate::view::static_types::Static<V> {
    type AsyncOutput = Self;
    type State = ();
    type Cloneable = Self;
    type CloneableOwned = Self;

    fn to_html(self, style: &mut String) {
        style.push_str(V);
        style.push(';');
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        _el: &crate::renderer::types::Element,
    ) -> Self::State {
    }

    fn build(self, el: &crate::renderer::types::Element) -> Self::State {
        Rndr::set_attribute(el, "style", V);
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
}

/*
#[cfg(test)]
mod tests {
    use crate::{
        html::{
            element::{p, HtmlElement},
            style::style,
        },
        renderer::dom::Dom,
        view::{Position, PositionState, RenderHtml},
    };

    #[test]
    fn adds_simple_style() {
        let mut html = String::new();
        let el: HtmlElement<_, _, _, Dom> = p(style("display: block"), ());
        el.to_html(&mut html, &PositionState::new(Position::FirstChild));

        assert_eq!(html, r#"<p style="display: block;"></p>"#);
    }

    #[test]
    fn mixes_plain_and_specific_styles() {
        let mut html = String::new();
        let el: HtmlElement<_, _, _, Dom> =
            p((style("display: block"), style(("color", "blue"))), ());
        el.to_html(&mut html, &PositionState::new(Position::FirstChild));

        assert_eq!(html, r#"<p style="display: block;color:blue;"></p>"#);
    }

    #[test]
    fn handles_dynamic_styles() {
        let mut html = String::new();
        let el: HtmlElement<_, _, _, Dom> = p(
            (
                style("display: block"),
                style(("color", "blue")),
                style(("font-weight", || "bold".to_string())),
            ),
            (),
        );
        el.to_html(&mut html, &PositionState::new(Position::FirstChild));

        assert_eq!(
            html,
            r#"<p style="display: block;color:blue;font-weight:bold;"></p>"#
        );
    }
}
 */
