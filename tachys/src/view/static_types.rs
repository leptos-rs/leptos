use super::{
    Mountable, NeverError, Position, PositionState, Render, RenderHtml,
    ToTemplate,
};
use crate::{
    html::{
        attribute::{Attribute, AttributeKey, AttributeValue},
        class::IntoClass,
        style::IntoStyle,
    },
    hydration::Cursor,
    renderer::{DomRenderer, Renderer},
};
use std::marker::PhantomData;

/// An attribute for which both the key and the value are known at compile time,
/// i.e., as `&'static str`s.
///
/// ```
/// use tachydom::{
///     html::attribute::{Attribute, Type},
///     view::static_types::{static_attr, StaticAttr},
/// };
/// let input_type = static_attr::<Type, "text">();
/// let mut buf = String::new();
/// let mut classes = String::new();
/// let mut styles = String::new();
/// input_type.to_html(&mut buf, &mut classes, &mut styles);
/// assert_eq!(buf, " type=\"text\"");
/// ```
#[derive(Debug)]
pub struct StaticAttr<K: AttributeKey, const V: &'static str> {
    ty: PhantomData<K>,
}

impl<K: AttributeKey, const V: &'static str> PartialEq for StaticAttr<K, V> {
    fn eq(&self, _other: &Self) -> bool {
        // by definition, two static attrs with same key and same const V are same
        true
    }
}

pub fn static_attr<K: AttributeKey, const V: &'static str>() -> StaticAttr<K, V>
{
    StaticAttr { ty: PhantomData }
}

impl<K, const V: &'static str> ToTemplate for StaticAttr<K, V>
where
    K: AttributeKey,
{
    fn to_template(
        buf: &mut String,
        _class: &mut String,
        _style: &mut String,
        _inner_html: &mut String,
        _position: &mut Position,
    ) {
        buf.push(' ');
        buf.push_str(K::KEY);
        buf.push_str("=\"");
        buf.push_str(V);
        buf.push('"');
    }
}

impl<K, const V: &'static str, R> Attribute<R> for StaticAttr<K, V>
where
    K: AttributeKey,
    R: Renderer,
    R::Element: Clone,
{
    const MIN_LENGTH: usize = K::KEY.len() + 3 + V.len(); // K::KEY + ="..." + V
    type State = ();

    fn to_html(
        self,
        buf: &mut String,
        _class: &mut String,
        _style: &mut String,
        _inner_html: &mut String,
    ) {
        AttributeValue::<R>::to_html(V, K::KEY, buf)
    }

    fn hydrate<const FROM_SERVER: bool>(self, _el: &R::Element) -> Self::State {
    }

    fn build(self, el: &R::Element) -> Self::State {
        R::set_attribute(el, K::KEY, V);
    }

    fn rebuild(self, _state: &mut Self::State) {}
}

#[derive(Debug)]
pub struct Static<const V: &'static str>;

impl<const V: &'static str> PartialEq for Static<V> {
    fn eq(&self, _other: &Self) -> bool {
        // by definition, two static values of same const V are same
        true
    }
}

impl<const V: &'static str> AsRef<str> for Static<V> {
    fn as_ref(&self) -> &str {
        V
    }
}

impl<const V: &'static str, R: Renderer> Render<R> for Static<V>
where
    R::Text: Mountable<R>,
{
    type State = Option<R::Text>;
    type FallibleState = Self::State;
    type Error = NeverError;

    fn build(self) -> Self::State {
        // a view state has to be returned so it can be mounted
        Some(R::create_text_node(V))
    }

    // This type is specified as static, so no rebuilding is done.
    fn rebuild(self, _state: &mut Self::State) {}

    fn try_build(self) -> Result<Self::FallibleState, Self::Error> {
        Ok(Render::<R>::build(self))
    }

    fn try_rebuild(
        self,
        state: &mut Self::FallibleState,
    ) -> Result<(), Self::Error> {
        Ok(Render::<R>::rebuild(self, state))
    }
}

impl<const V: &'static str, R> RenderHtml<R> for Static<V>
where
    R: Renderer,
    R::Node: Clone,
    R::Element: Clone,
    R::Text: Mountable<R>,
{
    const MIN_LENGTH: usize = V.len();

    fn to_html_with_buf(self, buf: &mut String, position: &mut Position) {
        // add a comment node to separate from previous sibling, if any
        if matches!(position, Position::NextChildAfterText) {
            buf.push_str("<!>")
        }
        buf.push_str(V);
        *position = Position::NextChildAfterText;
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        cursor: &Cursor<R>,
        position: &PositionState,
    ) -> Self::State {
        if position.get() == Position::FirstChild {
            cursor.child();
        } else {
            cursor.sibling();
        }
        if matches!(position.get(), Position::NextChildAfterText) {
            cursor.sibling();
        }
        position.set(Position::NextChildAfterText);

        // no view state is created when hydrating, because this is static
        None
    }
}

impl<const V: &'static str> ToTemplate for Static<V> {
    const TEMPLATE: &'static str = V;

    fn to_template(
        buf: &mut String,
        _class: &mut String,
        _style: &mut String,
        _inner_html: &mut String,
        position: &mut Position,
    ) {
        if matches!(*position, Position::NextChildAfterText) {
            buf.push_str("<!>")
        }
        buf.push_str(V);
        *position = Position::NextChildAfterText;
    }
}
