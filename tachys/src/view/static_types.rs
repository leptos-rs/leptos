use super::{
    add_attr::AddAnyAttr, Mountable, Position, PositionState, Render,
    RenderHtml, ToTemplate,
};
use crate::{
    html::attribute::{
        maybe_next_attr_erasure_macros::{
            next_attr_combine, next_attr_output_type,
        },
        Attribute, AttributeKey, AttributeValue, NextAttribute,
    },
    hydration::Cursor,
    renderer::{CastFrom, Rndr},
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
    next_attr_output_type!(Self, NewAttr);

    fn add_any_attr<NewAttr: Attribute>(
        self,
        new_attr: NewAttr,
    ) -> Self::Output<NewAttr> {
        next_attr_combine!(StaticAttr::<K, V> { ty: PhantomData }, new_attr)
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
        if V.is_empty() && escape {
            buf.push(' ');
        } else if escape {
            let escaped = html_escape::encode_text(V);
            buf.push_str(&escaped);
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

        // separating placeholder marker comes before text node
        if matches!(position.get(), Position::NextChildAfterText) {
            cursor.sibling();
        }

        let node = cursor.current();
        let node = crate::renderer::types::Text::cast_from(node.clone())
            .unwrap_or_else(|| {
                crate::hydration::failed_to_cast_text_node(node)
            });

        position.set(Position::NextChildAfterText);

        Some(node)
    }
}

impl<const V: &'static str> AddAnyAttr for Static<V> {
    type Output<NewAttr: Attribute> = Static<V>;

    fn add_any_attr<NewAttr: Attribute>(
        self,
        _attr: NewAttr,
    ) -> Self::Output<NewAttr>
    where
        Self::Output<NewAttr>: RenderHtml,
    {
        // inline helper function to assist the compiler with type inference
        #[inline(always)]
        const fn create_static<const S: &'static str, A: Attribute>(
        ) -> <Static<S> as AddAnyAttr>::Output<A> {
            Static
        }

        // call the helper function with the current const value and new attribute type
        create_static::<V, NewAttr>()
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
