use crate::renderer::Rndr;
use std::{
    borrow::Cow,
    future::Future,
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6},
    num::{
        NonZeroI128, NonZeroI16, NonZeroI32, NonZeroI64, NonZeroI8,
        NonZeroIsize, NonZeroU128, NonZeroU16, NonZeroU32, NonZeroU64,
        NonZeroU8, NonZeroUsize,
    },
    sync::Arc,
};

/// Declares that this type can be converted into some other type, which is a valid attribute value.
pub trait IntoAttributeValue {
    /// The attribute value into which this type can be converted.
    type Output;

    /// Consumes this value, transforming it into an attribute value.
    fn into_attribute_value(self) -> Self::Output;
}

impl<T> IntoAttributeValue for T
where
    T: AttributeValue,
{
    type Output = Self;

    fn into_attribute_value(self) -> Self::Output {
        self
    }
}

/// A possible value for an HTML attribute.
pub trait AttributeValue: Send {
    /// The state that should be retained between building and rebuilding.
    type State;

    /// The type once all async data have loaded.
    type AsyncOutput: AttributeValue;

    /// A version of the value that can be cloned. This can be the same type, or a
    /// reference-counted type. Generally speaking, this does *not* need to refer to the same data,
    /// but should behave in the same way. So for example, making an event handler cloneable should
    /// probably make it reference-counted (so that a `FnMut()` continues mutating the same
    /// closure), but making a `String` cloneable does not necessarily need to make it an
    /// `Arc<str>`, as two different clones of a `String` will still have the same value.
    type Cloneable: AttributeValue + Clone;

    /// A cloneable type that is also `'static`. This is used for spreading across types when the
    /// spreadable attribute needs to be owned. In some cases (`&'a str` to `Arc<str>`, etc.) the owned
    /// cloneable type has worse performance than the cloneable type, so they are separate.
    type CloneableOwned: AttributeValue + Clone + 'static;

    /// An approximation of the actual length of this attribute in HTML.
    fn html_len(&self) -> usize;

    /// Renders the attribute value to HTML.
    fn to_html(self, key: &str, buf: &mut String);

    /// Renders the attribute value to HTML for a `<template>`.
    fn to_template(key: &str, buf: &mut String);

    /// Adds interactivity as necessary, given DOM nodes that were created from HTML that has
    /// either been rendered on the server, or cloned for a `<template>`.
    fn hydrate<const FROM_SERVER: bool>(
        self,
        key: &str,
        el: &crate::renderer::types::Element,
    ) -> Self::State;

    /// Adds this attribute to the element during client-side rendering.
    fn build(
        self,
        el: &crate::renderer::types::Element,
        key: &str,
    ) -> Self::State;

    /// Applies a new value for the attribute.
    fn rebuild(self, key: &str, state: &mut Self::State);

    /// Converts this attribute into an equivalent that can be cloned.
    fn into_cloneable(self) -> Self::Cloneable;

    /// Converts this attributes into an equivalent that can be cloned and is `'static`.
    fn into_cloneable_owned(self) -> Self::CloneableOwned;

    /// “Runs” the attribute without other side effects. For primitive types, this is a no-op. For
    /// reactive types, this can be used to gather data about reactivity or about asynchronous data
    /// that needs to be loaded.
    fn dry_resolve(&mut self);

    /// “Resolves” this into a form that is not waiting for any asynchronous data.
    fn resolve(self) -> impl Future<Output = Self::AsyncOutput> + Send;
}

impl AttributeValue for () {
    type State = ();
    type AsyncOutput = ();
    type Cloneable = ();
    type CloneableOwned = ();

    fn html_len(&self) -> usize {
        0
    }

    fn to_html(self, _key: &str, _buf: &mut String) {}

    fn to_template(_key: &str, _buf: &mut String) {}

    fn hydrate<const FROM_SERVER: bool>(
        self,
        _key: &str,
        _el: &crate::renderer::types::Element,
    ) {
    }

    fn build(
        self,
        _el: &crate::renderer::types::Element,
        _key: &str,
    ) -> Self::State {
    }

    fn rebuild(self, _key: &str, _state: &mut Self::State) {}

    fn into_cloneable(self) -> Self::Cloneable {
        self
    }

    fn into_cloneable_owned(self) -> Self::CloneableOwned {
        self
    }

    fn dry_resolve(&mut self) {}

    async fn resolve(self) {}
}

impl<'a> AttributeValue for &'a str {
    type State = (crate::renderer::types::Element, &'a str);
    type AsyncOutput = &'a str;
    type Cloneable = &'a str;
    type CloneableOwned = Arc<str>;

    fn html_len(&self) -> usize {
        self.len()
    }

    fn to_html(self, key: &str, buf: &mut String) {
        buf.push(' ');
        buf.push_str(key);
        buf.push_str("=\"");
        buf.push_str(&escape_attr(self));
        buf.push('"');
    }

    fn to_template(_key: &str, _buf: &mut String) {}

    fn hydrate<const FROM_SERVER: bool>(
        self,
        key: &str,
        el: &crate::renderer::types::Element,
    ) -> Self::State {
        // if we're actually hydrating from SSRed HTML, we don't need to set the attribute
        // if we're hydrating from a CSR-cloned <template>, we do need to set non-StaticAttr attributes
        if !FROM_SERVER {
            Rndr::set_attribute(el, key, self);
        }
        (el.clone(), self)
    }

    fn build(
        self,
        el: &crate::renderer::types::Element,
        key: &str,
    ) -> Self::State {
        Rndr::set_attribute(el, key, self);
        (el.to_owned(), self)
    }

    fn rebuild(self, key: &str, state: &mut Self::State) {
        let (el, prev_value) = state;
        if self != *prev_value {
            Rndr::set_attribute(el, key, self);
        }
        *prev_value = self;
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

#[cfg(feature = "nightly")]
impl<const V: &'static str> AttributeValue
    for crate::view::static_types::Static<V>
{
    type AsyncOutput = Self;
    type State = ();
    type Cloneable = Self;
    type CloneableOwned = Self;

    fn html_len(&self) -> usize {
        V.len()
    }

    fn to_html(self, key: &str, buf: &mut String) {
        <&str as AttributeValue>::to_html(V, key, buf);
    }

    fn to_template(key: &str, buf: &mut String) {
        buf.push(' ');
        buf.push_str(key);
        buf.push_str("=\"");
        buf.push_str(V);
        buf.push('"');
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        _key: &str,
        _el: &crate::renderer::types::Element,
    ) -> Self::State {
    }

    fn build(
        self,
        el: &crate::renderer::types::Element,
        key: &str,
    ) -> Self::State {
        <&str as AttributeValue>::build(V, el, key);
    }

    fn rebuild(self, _key: &str, _state: &mut Self::State) {}

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

impl<'a> AttributeValue for &'a String {
    type AsyncOutput = Self;
    type State = (crate::renderer::types::Element, &'a String);
    type Cloneable = Self;
    type CloneableOwned = Arc<str>;

    fn html_len(&self) -> usize {
        self.len()
    }

    fn to_html(self, key: &str, buf: &mut String) {
        <&str as AttributeValue>::to_html(self.as_str(), key, buf);
    }

    fn to_template(_key: &str, _buf: &mut String) {}

    fn hydrate<const FROM_SERVER: bool>(
        self,
        key: &str,
        el: &crate::renderer::types::Element,
    ) -> Self::State {
        let (el, _) = <&str as AttributeValue>::hydrate::<FROM_SERVER>(
            self.as_str(),
            key,
            el,
        );
        (el, self)
    }

    fn build(
        self,
        el: &crate::renderer::types::Element,
        key: &str,
    ) -> Self::State {
        Rndr::set_attribute(el, key, self);
        (el.clone(), self)
    }

    fn rebuild(self, key: &str, state: &mut Self::State) {
        let (el, prev_value) = state;
        if self != *prev_value {
            Rndr::set_attribute(el, key, self);
        }
        *prev_value = self;
    }

    fn into_cloneable(self) -> Self::Cloneable {
        self
    }

    fn into_cloneable_owned(self) -> Self::CloneableOwned {
        self.as_str().into()
    }

    fn dry_resolve(&mut self) {}

    async fn resolve(self) -> Self::AsyncOutput {
        self
    }
}

impl AttributeValue for String {
    type AsyncOutput = Self;
    type State = (crate::renderer::types::Element, String);
    type Cloneable = Arc<str>;
    type CloneableOwned = Arc<str>;

    fn html_len(&self) -> usize {
        self.len()
    }

    fn to_html(self, key: &str, buf: &mut String) {
        <&str as AttributeValue>::to_html(self.as_str(), key, buf);
    }

    fn to_template(_key: &str, _buf: &mut String) {}

    fn hydrate<const FROM_SERVER: bool>(
        self,
        key: &str,
        el: &crate::renderer::types::Element,
    ) -> Self::State {
        let (el, _) = <&str as AttributeValue>::hydrate::<FROM_SERVER>(
            self.as_str(),
            key,
            el,
        );
        (el, self)
    }

    fn build(
        self,
        el: &crate::renderer::types::Element,
        key: &str,
    ) -> Self::State {
        Rndr::set_attribute(el, key, &self);
        (el.clone(), self)
    }

    fn rebuild(self, key: &str, state: &mut Self::State) {
        let (el, prev_value) = state;
        if self != *prev_value {
            Rndr::set_attribute(el, key, &self);
        }
        *prev_value = self;
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

impl AttributeValue for Arc<str> {
    type AsyncOutput = Self;
    type State = (crate::renderer::types::Element, Arc<str>);
    type Cloneable = Arc<str>;
    type CloneableOwned = Arc<str>;

    fn html_len(&self) -> usize {
        self.len()
    }

    fn to_html(self, key: &str, buf: &mut String) {
        <&str as AttributeValue>::to_html(self.as_ref(), key, buf);
    }

    fn to_template(_key: &str, _buf: &mut String) {}

    fn hydrate<const FROM_SERVER: bool>(
        self,
        key: &str,
        el: &crate::renderer::types::Element,
    ) -> Self::State {
        let (el, _) = <&str as AttributeValue>::hydrate::<FROM_SERVER>(
            self.as_ref(),
            key,
            el,
        );
        (el, self)
    }

    fn build(
        self,
        el: &crate::renderer::types::Element,
        key: &str,
    ) -> Self::State {
        Rndr::set_attribute(el, key, &self);
        (el.clone(), self)
    }

    fn rebuild(self, key: &str, state: &mut Self::State) {
        let (el, prev_value) = state;
        if self != *prev_value {
            Rndr::set_attribute(el, key, &self);
        }
        *prev_value = self;
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
// TODO impl AttributeValue for Rc<str> and Arc<str> too

impl AttributeValue for bool {
    type AsyncOutput = Self;
    type State = (crate::renderer::types::Element, bool);
    type Cloneable = Self;
    type CloneableOwned = Self;

    fn html_len(&self) -> usize {
        0
    }

    fn to_html(self, key: &str, buf: &mut String) {
        if self {
            buf.push(' ');
            buf.push_str(key);
        }
    }

    fn to_template(_key: &str, _buf: &mut String) {}

    fn hydrate<const FROM_SERVER: bool>(
        self,
        key: &str,
        el: &crate::renderer::types::Element,
    ) -> Self::State {
        // if we're actually hydrating from SSRed HTML, we don't need to set the attribute
        // if we're hydrating from a CSR-cloned <template>, we do need to set non-StaticAttr attributes
        if !FROM_SERVER {
            Rndr::set_attribute(el, key, "");
        }
        (el.clone(), self)
    }

    fn build(
        self,
        el: &crate::renderer::types::Element,
        key: &str,
    ) -> Self::State {
        if self {
            Rndr::set_attribute(el, key, "");
        }
        (el.clone(), self)
    }

    fn rebuild(self, key: &str, state: &mut Self::State) {
        let (el, prev_value) = state;
        if self != *prev_value {
            if self {
                Rndr::set_attribute(el, key, "");
            } else {
                Rndr::remove_attribute(el, key);
            }
        }
        *prev_value = self;
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

impl<V> AttributeValue for Option<V>
where
    V: AttributeValue,
{
    type AsyncOutput = Option<V::AsyncOutput>;
    type State = (crate::renderer::types::Element, Option<V::State>);
    type Cloneable = Option<V::Cloneable>;
    type CloneableOwned = Option<V::CloneableOwned>;

    fn html_len(&self) -> usize {
        match self {
            Some(i) => i.html_len(),
            None => 0,
        }
    }

    fn to_html(self, key: &str, buf: &mut String) {
        if let Some(v) = self {
            v.to_html(key, buf);
        }
    }

    fn to_template(_key: &str, _buf: &mut String) {}

    fn hydrate<const FROM_SERVER: bool>(
        self,
        key: &str,
        el: &crate::renderer::types::Element,
    ) -> Self::State {
        let state = self.map(|v| v.hydrate::<FROM_SERVER>(key, el));
        (el.clone(), state)
    }

    fn build(
        self,
        el: &crate::renderer::types::Element,
        key: &str,
    ) -> Self::State {
        let el = el.clone();
        let v = self.map(|v| v.build(&el, key));
        (el, v)
    }

    fn rebuild(self, key: &str, state: &mut Self::State) {
        let (el, prev) = state;
        match (self, prev.as_mut()) {
            (None, None) => {}
            (None, Some(_)) => {
                Rndr::remove_attribute(el, key);
                *prev = None;
            }
            (Some(value), None) => {
                *prev = Some(value.build(el, key));
            }
            (Some(new), Some(old)) => {
                new.rebuild(key, old);
            }
        }
    }

    fn into_cloneable(self) -> Self::Cloneable {
        self.map(|value| value.into_cloneable())
    }

    fn into_cloneable_owned(self) -> Self::CloneableOwned {
        self.map(|value| value.into_cloneable_owned())
    }

    fn dry_resolve(&mut self) {
        if let Some(inner) = self.as_mut() {
            inner.dry_resolve();
        }
    }

    async fn resolve(self) -> Self::AsyncOutput {
        match self {
            None => None,
            Some(inner) => Some(inner.resolve().await),
        }
    }
}

pub(crate) fn escape_attr(value: &str) -> Cow<'_, str> {
    html_escape::encode_double_quoted_attribute(value)
}

macro_rules! render_primitive {
  ($($child_type:ty),* $(,)?) => {
      $(
        impl AttributeValue for $child_type
        where

        {
            type AsyncOutput = $child_type;
            type State = (crate::renderer::types::Element, $child_type);
            type Cloneable = Self;
            type CloneableOwned = Self;

            fn html_len(&self) -> usize {
                0
            }

            fn to_html(self, key: &str, buf: &mut String) {
                <String as AttributeValue>::to_html(self.to_string(), key, buf);
            }

            fn to_template(_key: &str, _buf: &mut String) {}

            fn hydrate<const FROM_SERVER: bool>(
                self,
                key: &str,
                el: &crate::renderer::types::Element,
            ) -> Self::State {
                // if we're actually hydrating from SSRed HTML, we don't need to set the attribute
                // if we're hydrating from a CSR-cloned <template>, we do need to set non-StaticAttr attributes
                if !FROM_SERVER {
                    Rndr::set_attribute(el, key, &self.to_string());
                }
                (el.clone(), self)
            }

            fn build(self, el: &crate::renderer::types::Element, key: &str) -> Self::State {
                Rndr::set_attribute(el, key, &self.to_string());
                (el.to_owned(), self)
            }

            fn rebuild(self, key: &str, state: &mut Self::State) {
                let (el, prev_value) = state;
                if self != *prev_value {
                    Rndr::set_attribute(el, key, &self.to_string());
                }
                *prev_value = self;
            }

            fn into_cloneable(self) -> Self::Cloneable {
                self
            }

            fn into_cloneable_owned(self) -> Self::CloneableOwned {
                self
            }

            fn dry_resolve(&mut self) {
            }

            async fn resolve(self) -> Self::AsyncOutput {
                self
            }
        }
      )*
  }
}

render_primitive![
    usize,
    u8,
    u16,
    u32,
    u64,
    u128,
    isize,
    i8,
    i16,
    i32,
    i64,
    i128,
    f32,
    f64,
    char,
    IpAddr,
    SocketAddr,
    SocketAddrV4,
    SocketAddrV6,
    Ipv4Addr,
    Ipv6Addr,
    NonZeroI8,
    NonZeroU8,
    NonZeroI16,
    NonZeroU16,
    NonZeroI32,
    NonZeroU32,
    NonZeroI64,
    NonZeroU64,
    NonZeroI128,
    NonZeroU128,
    NonZeroIsize,
    NonZeroUsize,
];
