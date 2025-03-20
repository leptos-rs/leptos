use super::attribute::{
    maybe_next_attr_erasure_macros::next_attr_output_type, Attribute,
    NextAttribute,
};
#[cfg(all(feature = "nightly", rustc_nightly))]
use crate::view::static_types::Static;
use crate::{
    html::attribute::maybe_next_attr_erasure_macros::next_attr_combine,
    renderer::{dom::CssStyleDeclaration, Rndr},
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
    next_attr_output_type!(Self, NewAttr);

    fn add_any_attr<NewAttr: Attribute>(
        self,
        new_attr: NewAttr,
    ) -> Self::Output<NewAttr> {
        next_attr_combine!(self, new_attr)
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
/// the [`CssStyleDeclaration`](web_sys::CssStyleDeclaration).
///
/// This could be a plain string, or a property name-value pair.
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

    /// Reset the styling to the state before this style was added.
    fn reset(state: &mut Self::State);
}

impl<T: IntoStyle> IntoStyle for Option<T> {
    type AsyncOutput = Option<T::AsyncOutput>;
    type State = (crate::renderer::types::Element, Option<T::State>);
    type Cloneable = Option<T::Cloneable>;
    type CloneableOwned = Option<T::CloneableOwned>;

    fn to_html(self, style: &mut String) {
        if let Some(t) = self {
            t.to_html(style);
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

    fn reset(state: &mut Self::State) {
        let (el, _prev) = state;
        Rndr::remove_attribute(el, "style");
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

    fn reset(state: &mut Self::State) {
        let (el, _prev) = state;
        Rndr::remove_attribute(el, "style");
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

    fn reset(state: &mut Self::State) {
        let (el, _prev) = state;
        Rndr::remove_attribute(el, "style");
    }
}

/// Any type that can be used to set an individual style in the
/// [`CssStyleDeclaration`](web_sys::CssStyleDeclaration).
///
/// This is the value in a `(name, value)` tuple that implements [`IntoStyle`].
pub trait IntoStyleValue: Send {
    /// The type after all async data have resolved.
    type AsyncOutput: IntoStyleValue;
    /// The view state retained between building and rebuilding.
    type State;
    /// An equivalent value that can be cloned.
    type Cloneable: Clone + IntoStyleValue;
    /// An equivalent value that can be cloned and is `'static`.
    type CloneableOwned: Clone + IntoStyleValue + 'static;

    /// Renders the style to HTML.
    fn to_html(self, name: &str, style: &mut String);

    /// Adds this style to the element during client-side rendering.
    fn build(self, style: &CssStyleDeclaration, name: &str) -> Self::State;

    /// Updates the value.
    fn rebuild(
        self,
        style: &CssStyleDeclaration,
        name: &str,
        state: &mut Self::State,
    );

    /// Adds interactivity as necessary, given DOM nodes that were created from HTML that has
    /// either been rendered on the server, or cloned for a `<template>`.
    fn hydrate(self, style: &CssStyleDeclaration, name: &str) -> Self::State;

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

impl<K, V> IntoStyle for (K, V)
where
    K: AsRef<str> + Clone + Send + 'static,
    V: IntoStyleValue,
{
    type AsyncOutput = (K, V::AsyncOutput);
    type State = (crate::renderer::types::CssStyleDeclaration, K, V::State);
    type Cloneable = (K, V::Cloneable);
    type CloneableOwned = (K, V::CloneableOwned);

    fn to_html(self, style: &mut String) {
        let (name, value) = self;
        value.to_html(name.as_ref(), style);
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        el: &crate::renderer::types::Element,
    ) -> Self::State {
        let style = Rndr::style(el);
        let state = self.1.hydrate(&style, self.0.as_ref());
        (style, self.0, state)
    }

    fn build(self, el: &crate::renderer::types::Element) -> Self::State {
        let (name, value) = self;
        let style = Rndr::style(el);
        let state = value.build(&style, name.as_ref());
        (style, name, state)
    }

    fn rebuild(self, state: &mut Self::State) {
        let (name, value) = self;
        // state.1 was the previous name, theoretically the css name could be changed:
        if name.as_ref() != state.1.as_ref() {
            <Self as IntoStyle>::reset(state);
            state.2 = value.build(&state.0, name.as_ref());
        } else {
            value.rebuild(&state.0, name.as_ref(), &mut state.2);
        }
    }

    fn into_cloneable(self) -> Self::Cloneable {
        (self.0, self.1.into_cloneable())
    }

    fn into_cloneable_owned(self) -> Self::CloneableOwned {
        (self.0, self.1.into_cloneable_owned())
    }

    fn dry_resolve(&mut self) {
        self.1.dry_resolve();
    }

    async fn resolve(self) -> Self::AsyncOutput {
        (self.0, self.1.resolve().await)
    }

    /// Reset the renderer to the state before this style was added.
    fn reset(state: &mut Self::State) {
        let (style, name, _value) = state;
        Rndr::remove_css_property(style, name.as_ref());
    }
}

macro_rules! impl_style_value {
    ($ty:ty) => {
        impl IntoStyleValue for $ty {
            type AsyncOutput = Self;
            type State = Self;
            type Cloneable = Self;
            type CloneableOwned = Self;

            fn to_html(self, name: &str, style: &mut String) {
                style.push_str(name);
                style.push(':');
                style.push_str(&self);
                style.push(';');
            }

            fn build(
                self,
                style: &CssStyleDeclaration,
                name: &str,
            ) -> Self::State {
                Rndr::set_css_property(style, name, &self);
                self
            }

            fn rebuild(
                self,
                style: &CssStyleDeclaration,
                name: &str,
                state: &mut Self::State,
            ) {
                if &self != &*state {
                    Rndr::set_css_property(style, name, &self);
                }
                *state = self;
            }

            fn hydrate(
                self,
                _style: &CssStyleDeclaration,
                _name: &str,
            ) -> Self::State {
                self
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

        impl IntoStyleValue for Option<$ty> {
            type AsyncOutput = Self;
            type State = Self;
            type Cloneable = Self;
            type CloneableOwned = Self;

            fn to_html(self, name: &str, style: &mut String) {
                if let Some(value) = self {
                    style.push_str(name);
                    style.push(':');
                    style.push_str(&value);
                    style.push(';');
                }
            }

            fn build(
                self,
                style: &CssStyleDeclaration,
                name: &str,
            ) -> Self::State {
                if let Some(value) = &self {
                    Rndr::set_css_property(style, name, &value);
                }
                self
            }

            fn rebuild(
                self,
                style: &CssStyleDeclaration,
                name: &str,
                state: &mut Self::State,
            ) {
                match (&state, &self) {
                    (None, None) => {}
                    (Some(_), None) => Rndr::remove_css_property(style, name),
                    (None, Some(value)) => {
                        Rndr::set_css_property(style, name, &value)
                    }
                    (Some(old), Some(new)) => {
                        if new != &*old {
                            Rndr::set_css_property(style, name, &new);
                        }
                    }
                }
                *state = self;
            }

            fn hydrate(
                self,
                _style: &CssStyleDeclaration,
                _name: &str,
            ) -> Self::State {
                self
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
    };
}

impl_style_value!(&'static str);
impl_style_value!(Arc<str>);
impl_style_value!(String);
#[cfg(feature = "oco")]
impl_style_value!(oco_ref::Oco<'static, str>);

#[cfg(all(feature = "nightly", rustc_nightly))]
impl<const V: &'static str> IntoStyleValue for Static<V> {
    type AsyncOutput = Self;
    type State = Self;
    type Cloneable = Self;
    type CloneableOwned = Self;

    fn to_html(self, name: &str, style: &mut String) {
        style.push_str(name);
        style.push(':');
        style.push_str(V);
        style.push(';');
    }

    fn build(self, style: &CssStyleDeclaration, name: &str) -> Self::State {
        Rndr::set_css_property(style, name, V);
        self
    }

    fn rebuild(
        self,
        _style: &CssStyleDeclaration,
        _name: &str,
        _state: &mut Self::State,
    ) {
    }

    fn hydrate(self, _style: &CssStyleDeclaration, _name: &str) -> Self::State {
        self
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

#[cfg(all(feature = "nightly", rustc_nightly))]
impl<const V: &'static str> IntoStyleValue for Option<Static<V>> {
    type AsyncOutput = Self;
    type State = Self;
    type Cloneable = Self;
    type CloneableOwned = Self;

    fn to_html(self, name: &str, style: &mut String) {
        if self.is_some() {
            style.push_str(name);
            style.push(':');
            style.push_str(V);
            style.push(';');
        }
    }

    fn build(self, style: &CssStyleDeclaration, name: &str) -> Self::State {
        if self.is_some() {
            Rndr::set_css_property(style, name, V);
        }
        self
    }

    fn rebuild(
        self,
        style: &CssStyleDeclaration,
        name: &str,
        state: &mut Self::State,
    ) {
        match (&state, &self) {
            (None, None) => {}
            (Some(_), None) => Rndr::remove_css_property(style, name),
            (None, Some(_)) => Rndr::set_css_property(style, name, V),
            (Some(_), Some(_)) => {}
        }
        *state = self;
    }

    fn hydrate(self, _style: &CssStyleDeclaration, _name: &str) -> Self::State {
        self
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

#[cfg(all(feature = "nightly", rustc_nightly))]
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

    fn reset(_state: &mut Self::State) {}
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
