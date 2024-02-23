use super::attribute::{Attribute, NextAttribute};
use crate::{
    renderer::DomRenderer,
    view::{Position, ToTemplate},
};
use std::{borrow::Cow, marker::PhantomData};

/// Adds to the style attribute of the parent element.
///
/// This can take a plain string value, which will be assigned to the `style`
#[inline(always)]
pub fn style<S, R>(style: S) -> Style<S, R>
where
    S: IntoStyle<R>,
    R: DomRenderer,
{
    Style {
        style,
        rndr: PhantomData,
    }
}

pub struct Style<S, R>
where
    S: IntoStyle<R>,
    R: DomRenderer,
{
    style: S,
    rndr: PhantomData<R>,
}

impl<S, R> Attribute<R> for Style<S, R>
where
    S: IntoStyle<R>,
    R: DomRenderer,
{
    const MIN_LENGTH: usize = 0;
    type State = S::State;

    fn to_html(
        self,
        _buf: &mut String,
        _style: &mut String,
        style: &mut String,
        _inner_html: &mut String,
    ) {
        self.style.to_html(style);
    }

    fn hydrate<const FROM_SERVER: bool>(self, el: &R::Element) -> Self::State {
        self.style.hydrate::<FROM_SERVER>(el)
    }

    fn build(self, el: &R::Element) -> Self::State {
        self.style.build(el)
    }

    fn rebuild(self, state: &mut Self::State) {
        self.style.rebuild(state)
    }
}

impl<S, R> NextAttribute<R> for Style<S, R>
where
    S: IntoStyle<R>,
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

impl<S, R> ToTemplate for Style<S, R>
where
    S: IntoStyle<R>,
    R: DomRenderer,
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
pub trait IntoStyle<R: DomRenderer> {
    type State;

    fn to_html(self, style: &mut String);

    fn hydrate<const FROM_SERVER: bool>(self, el: &R::Element) -> Self::State;

    fn build(self, el: &R::Element) -> Self::State;

    fn rebuild(self, state: &mut Self::State);
}

pub trait StylePropertyValue<R: DomRenderer> {
    type State;

    fn to_html(self, name: &str, style: &mut String);

    fn hydrate<const FROM_SERVER: bool>(
        self,
        name: Cow<'static, str>,
        el: &R::Element,
    ) -> Self::State;

    fn rebuild(self, name: Cow<'static, str>, state: &mut Self::State);
}

impl<'a, R> IntoStyle<R> for &'a str
where
    R: DomRenderer,
{
    type State = (R::Element, &'a str);

    fn to_html(self, style: &mut String) {
        style.push_str(self);
        style.push(';');
    }

    fn hydrate<const FROM_SERVER: bool>(self, el: &R::Element) -> Self::State {
        (el.clone(), self)
    }

    fn build(self, el: &R::Element) -> Self::State {
        R::set_attribute(el, "style", self);
        (el.clone(), self)
    }

    fn rebuild(self, state: &mut Self::State) {
        let (el, prev) = state;
        if self != *prev {
            R::set_attribute(el, "style", self);
        }
        *prev = self;
    }
}

impl<R> IntoStyle<R> for String
where
    R: DomRenderer,
{
    type State = (R::Element, String);

    fn to_html(self, style: &mut String) {
        style.push_str(&self);
        style.push(';');
    }

    fn hydrate<const FROM_SERVER: bool>(self, el: &R::Element) -> Self::State {
        (el.clone(), self)
    }

    fn build(self, el: &R::Element) -> Self::State {
        R::set_attribute(el, "style", &self);
        (el.clone(), self)
    }

    fn rebuild(self, state: &mut Self::State) {
        let (el, prev) = state;
        if self != *prev {
            R::set_attribute(el, "style", &self);
        }
        *prev = self;
    }
}

impl<'a, R> IntoStyle<R> for (&'a str, &'a str)
where
    R: DomRenderer,
{
    type State = (R::CssStyleDeclaration, &'a str);

    fn to_html(self, style: &mut String) {
        let (name, value) = self;
        style.push_str(name);
        style.push(':');
        style.push_str(value);
        style.push(';');
    }

    fn hydrate<const FROM_SERVER: bool>(self, el: &R::Element) -> Self::State {
        let style = R::style(el);
        (style, self.1)
    }

    fn build(self, el: &R::Element) -> Self::State {
        let (name, value) = self;
        let style = R::style(el);
        R::set_css_property(&style, name, value);
        (style, self.1)
    }

    fn rebuild(self, state: &mut Self::State) {
        let (name, value) = self;
        let (style, prev) = state;
        if value != *prev {
            R::set_css_property(style, name, value);
        }
        *prev = value;
    }
}

impl<'a, R> IntoStyle<R> for (&'a str, String)
where
    R: DomRenderer,
{
    type State = (R::CssStyleDeclaration, String);

    fn to_html(self, style: &mut String) {
        let (name, value) = self;
        style.push_str(name);
        style.push(':');
        style.push_str(&value);
        style.push(';');
    }

    fn hydrate<const FROM_SERVER: bool>(self, el: &R::Element) -> Self::State {
        let style = R::style(el);
        (style, self.1)
    }

    fn build(self, el: &R::Element) -> Self::State {
        let (name, value) = &self;
        let style = R::style(el);
        R::set_css_property(&style, name, value);
        (style, self.1)
    }

    fn rebuild(self, state: &mut Self::State) {
        let (name, value) = self;
        let (style, prev) = state;
        if value != *prev {
            R::set_css_property(style, name, &value);
        }
        *prev = value;
    }
}

#[cfg(feature = "nightly")]
impl<const V: &'static str, R> IntoStyle<R>
    for crate::view::static_types::Static<V>
where
    R: DomRenderer,
{
    type State = ();

    fn to_html(self, style: &mut String) {
        style.push_str(V);
        style.push(';');
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        _el: &<R>::Element,
    ) -> Self::State {
    }

    fn build(self, el: &<R>::Element) -> Self::State {
        R::set_attribute(el, "style", V);
    }

    fn rebuild(self, _state: &mut Self::State) {}
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
