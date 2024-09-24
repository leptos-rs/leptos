use super::{
    add_attr::AddAnyAttr, Mountable, Position, PositionState, Render,
    RenderHtml, ToTemplate,
};
use crate::{
    html::attribute::{Attribute, AttributeKey, AttributeValue, NextAttribute},
    hydration::Cursor,
    renderer::Rndr,
};
use std::marker::PhantomData;

/// An attribute for which both the key and the value are known at compile time,
/// i.e., as `&'static str`s.
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

/// Creates an [`Attribute`] whose key and value are both known at compile time.
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

impl<K, const V: &'static str> Attribute for StaticAttr<K, V>
where
    K: AttributeKey,
{
    const MIN_LENGTH: usize = K::KEY.len() + 3 + V.len(); // K::KEY + ="..." + V

    type AsyncOutput = Self;
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
        AttributeValue::to_html(V, K::KEY, buf)
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        _el: &crate::renderer::types::Element,
    ) -> Self::State {
    }

    fn build(self, el: &crate::renderer::types::Element) -> Self::State {
        Rndr::set_attribute(el, K::KEY, V);
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

impl<K, const V: &'static str> NextAttribute for StaticAttr<K, V>
where
    K: AttributeKey,
{
    type Output<NewAttr: Attribute> = (Self, NewAttr);

    fn add_any_attr<NewAttr: Attribute>(
        self,
        new_attr: NewAttr,
    ) -> Self::Output<NewAttr> {
        (StaticAttr::<K, V> { ty: PhantomData }, new_attr)
    }
}

/// A static string that is known at compile time and can be optimized by including its type in the
/// view tree.
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

impl<const V: &'static str> Render for Static<V>
where
    crate::renderer::types::Text: Mountable,
{
    type State = Option<crate::renderer::types::Text>;

    fn build(self) -> Self::State {
        // a view state has to be returned so it can be mounted
        Some(Rndr::create_text_node(V))
    }

    // This type is specified as static, so no rebuilding is done.
    fn rebuild(self, _state: &mut Self::State) {}
}

impl<const V: &'static str> RenderHtml for Static<V> {
    type AsyncOutput = Self;

    const MIN_LENGTH: usize = V.len();

    fn dry_resolve(&mut self) {}

    // this won't actually compile because if a weird interaction because the const &'static str and
    // the RPITIT, so we just refine it to a concrete future type; this will never change in any
    // case
    #[allow(refining_impl_trait)]
    fn resolve(self) -> std::future::Ready<Self> {
        std::future::ready(self)
    }

    fn to_html_with_buf(
        self,
        buf: &mut String,
        position: &mut Position,
        escape: bool,
        _mark_branches: bool,
    ) {
        // add a comment node to separate from previous sibling, if any
        if matches!(position, Position::NextChildAfterText) {
            buf.push_str("<!>")
        }
        if escape {
            buf.push_str(&html_escape::encode_text(V));
        } else {
            buf.push_str(V);
        }
        *position = Position::NextChildAfterText;
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        cursor: &Cursor,
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

impl<const V: &'static str> AddAnyAttr for Static<V> {
    type Output<SomeNewAttr: Attribute> = Static<V>;

    fn add_any_attr<NewAttr: Attribute>(
        self,
        _attr: NewAttr,
    ) -> Self::Output<NewAttr>
    where
        Self::Output<NewAttr>: RenderHtml,
    {
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
