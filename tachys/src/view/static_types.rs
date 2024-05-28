use super::{
    add_attr::AddAnyAttr, Mountable, Position, PositionState, Render,
    RenderHtml, ToTemplate,
};
use crate::{
    html::attribute::{Attribute, AttributeKey, AttributeValue, NextAttribute},
    hydration::Cursor,
    renderer::Renderer,
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

impl<K: AttributeKey, const V: &'static str> Clone for StaticAttr<K, V> {
    fn clone(&self) -> Self {
        Self { ty: PhantomData }
    }
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
{
    const MIN_LENGTH: usize = K::KEY.len() + 3 + V.len(); // K::KEY + ="..." + V

    type State = ();
    type Cloneable = Self;
    type CloneableOwned = Self;

    #[inline(always)]
    fn html_len(&self) -> usize {
        K::KEY.len() + 3 + V.len()
    }

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

    fn into_cloneable(self) -> Self::Cloneable {
        self
    }

    fn into_cloneable_owned(self) -> Self::CloneableOwned {
        self
    }
}

impl<K, const V: &'static str, R> NextAttribute<R> for StaticAttr<K, V>
where
    K: AttributeKey,
    R: Renderer,
{
    type Output<NewAttr: Attribute<R>> = (Self, NewAttr);

    fn add_any_attr<NewAttr: Attribute<R>>(
        self,
        new_attr: NewAttr,
    ) -> Self::Output<NewAttr> {
        (StaticAttr::<K, V> { ty: PhantomData }, new_attr)
    }
}

#[derive(Debug, Clone, Copy)]
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

    fn build(self) -> Self::State {
        // a view state has to be returned so it can be mounted
        Some(R::create_text_node(V))
    }

    // This type is specified as static, so no rebuilding is done.
    fn rebuild(self, _state: &mut Self::State) {}
}

impl<const V: &'static str, R> RenderHtml<R> for Static<V>
where
    R: Renderer,

    R::Text: Mountable<R>,
{
    type AsyncOutput = Self;

    const MIN_LENGTH: usize = V.len();

    fn dry_resolve(&mut self) {}

    fn resolve(self) -> futures::future::Ready<Self::AsyncOutput> {
        futures::future::ready(self)
    }

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

impl<R, const V: &'static str> AddAnyAttr<R> for Static<V>
where
    R: Renderer,
{
    type Output<SomeNewAttr: Attribute<R>> = Static<V>;

    fn add_any_attr<NewAttr: Attribute<R>>(
        self,
        attr: NewAttr,
    ) -> Self::Output<NewAttr>
    where
        Self::Output<NewAttr>: RenderHtml<R>,
    {
        // TODO: there is a strange compiler thing that seems to prevent us returning Self here,
        // even though we've already said that Output is always the same as Self
        todo!()
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
