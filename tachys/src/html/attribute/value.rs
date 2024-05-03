use crate::renderer::Renderer;
use std::{
    borrow::Cow,
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6},
    num::{
        NonZeroI128, NonZeroI16, NonZeroI32, NonZeroI64, NonZeroI8,
        NonZeroIsize, NonZeroU128, NonZeroU16, NonZeroU32, NonZeroU64,
        NonZeroU8, NonZeroUsize,
    },
    sync::Arc,
};

pub trait AttributeValue<R: Renderer>: Send {
    type State;

    /// A version of the value that can be cloned. This can be the same type, or a
    /// reference-counted type. Generally speaking, this does *not* need to refer to the same data,
    /// but should behave in the same way. So for example, making an event handler cloneable should
    /// probably make it reference-counted (so that a `FnMut()` continues mutating the same
    /// closure), but making a `String` cloneable does not necessarily need to make it an
    /// `Arc<str>`, as two different clones of a `String` will still have the same value.
    type Cloneable: AttributeValue<R> + Clone;

    /// A cloneable type that is also `'static`. This is used for spreading across types when the
    /// spreadable attribute needs to be owned. In some cases (`&'a str` to `Arc<str>`, etc.) the owned
    /// cloneable type has worse performance than the cloneable type, so they are separate.
    type CloneableOwned: AttributeValue<R> + Clone + 'static;

    fn html_len(&self) -> usize;

    fn to_html(self, key: &str, buf: &mut String);

    fn to_template(key: &str, buf: &mut String);

    fn hydrate<const FROM_SERVER: bool>(
        self,
        key: &str,
        el: &R::Element,
    ) -> Self::State;

    fn build(self, el: &R::Element, key: &str) -> Self::State;

    fn rebuild(self, key: &str, state: &mut Self::State);

    fn into_cloneable(self) -> Self::Cloneable;

    fn into_cloneable_owned(self) -> Self::CloneableOwned;
}

impl<R> AttributeValue<R> for ()
where
    R: Renderer,
{
    type State = ();
    type Cloneable = ();
    type CloneableOwned = ();

    fn html_len(&self) -> usize {
        0
    }

    fn to_html(self, _key: &str, _buf: &mut String) {}

    fn to_template(_key: &str, _buf: &mut String) {}

    fn hydrate<const FROM_SERVER: bool>(self, _key: &str, _el: &R::Element) {}

    fn build(self, _el: &R::Element, _key: &str) -> Self::State {}

    fn rebuild(self, _key: &str, _state: &mut Self::State) {}

    fn into_cloneable(self) -> Self::Cloneable {
        self
    }

    fn into_cloneable_owned(self) -> Self::CloneableOwned {
        self
    }
}

impl<'a, R> AttributeValue<R> for &'a str
where
    R: Renderer,
{
    type State = (R::Element, &'a str);
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
        el: &R::Element,
    ) -> Self::State {
        // if we're actually hydrating from SSRed HTML, we don't need to set the attribute
        // if we're hydrating from a CSR-cloned <template>, we do need to set non-StaticAttr attributes
        if !FROM_SERVER {
            R::set_attribute(el, key, self);
        }
        (el.clone(), self)
    }

    fn build(self, el: &R::Element, key: &str) -> Self::State {
        R::set_attribute(el, key, self);
        (el.to_owned(), self)
    }

    fn rebuild(self, key: &str, state: &mut Self::State) {
        let (el, prev_value) = state;
        if self != *prev_value {
            R::set_attribute(el, key, self);
        }
        *prev_value = self;
    }

    fn into_cloneable(self) -> Self::Cloneable {
        self
    }

    fn into_cloneable_owned(self) -> Self::CloneableOwned {
        self.into()
    }
}

#[cfg(feature = "nightly")]
impl<R, const V: &'static str> AttributeValue<R>
    for crate::view::static_types::Static<V>
where
    R: Renderer,
{
    type State = ();
    type Cloneable = Self;
    type CloneableOwned = Self;

    fn html_len(&self) -> usize {
        V.len()
    }

    fn to_html(self, key: &str, buf: &mut String) {
        <&str as AttributeValue<R>>::to_html(V, key, buf);
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
        _el: &R::Element,
    ) -> Self::State {
    }

    fn build(self, el: &R::Element, key: &str) -> Self::State {
        <&str as AttributeValue<R>>::build(V, el, key);
    }

    fn rebuild(self, _key: &str, _state: &mut Self::State) {}

    fn into_cloneable(self) -> Self::Cloneable {
        self
    }

    fn into_cloneable_owned(self) -> Self::CloneableOwned {
        self
    }
}

impl<'a, R> AttributeValue<R> for &'a String
where
    R: Renderer,
{
    type State = (R::Element, &'a String);
    type Cloneable = Self;
    type CloneableOwned = Arc<str>;

    fn html_len(&self) -> usize {
        self.len()
    }

    fn to_html(self, key: &str, buf: &mut String) {
        <&str as AttributeValue<R>>::to_html(self.as_str(), key, buf);
    }

    fn to_template(_key: &str, _buf: &mut String) {}

    fn hydrate<const FROM_SERVER: bool>(
        self,
        key: &str,
        el: &R::Element,
    ) -> Self::State {
        let (el, _) = <&str as AttributeValue<R>>::hydrate::<FROM_SERVER>(
            self.as_str(),
            key,
            el,
        );
        (el, self)
    }

    fn build(self, el: &R::Element, key: &str) -> Self::State {
        R::set_attribute(el, key, self);
        (el.clone(), self)
    }

    fn rebuild(self, key: &str, state: &mut Self::State) {
        let (el, prev_value) = state;
        if self != *prev_value {
            R::set_attribute(el, key, self);
        }
        *prev_value = self;
    }

    fn into_cloneable(self) -> Self::Cloneable {
        self
    }

    fn into_cloneable_owned(self) -> Self::CloneableOwned {
        self.as_str().into()
    }
}

impl<R> AttributeValue<R> for String
where
    R: Renderer,
{
    type State = (R::Element, String);
    type Cloneable = Arc<str>;
    type CloneableOwned = Arc<str>;

    fn html_len(&self) -> usize {
        self.len()
    }

    fn to_html(self, key: &str, buf: &mut String) {
        <&str as AttributeValue<R>>::to_html(self.as_str(), key, buf);
    }

    fn to_template(_key: &str, _buf: &mut String) {}

    fn hydrate<const FROM_SERVER: bool>(
        self,
        key: &str,
        el: &R::Element,
    ) -> Self::State {
        let (el, _) = <&str as AttributeValue<R>>::hydrate::<FROM_SERVER>(
            self.as_str(),
            key,
            el,
        );
        (el, self)
    }

    fn build(self, el: &R::Element, key: &str) -> Self::State {
        R::set_attribute(el, key, &self);
        (el.clone(), self)
    }

    fn rebuild(self, key: &str, state: &mut Self::State) {
        let (el, prev_value) = state;
        if self != *prev_value {
            R::set_attribute(el, key, &self);
        }
        *prev_value = self;
    }

    fn into_cloneable(self) -> Self::Cloneable {
        self.into()
    }

    fn into_cloneable_owned(self) -> Self::CloneableOwned {
        self.into()
    }
}

impl<R> AttributeValue<R> for Arc<str>
where
    R: Renderer,
{
    type State = (R::Element, Arc<str>);
    type Cloneable = Arc<str>;
    type CloneableOwned = Arc<str>;

    fn html_len(&self) -> usize {
        self.len()
    }

    fn to_html(self, key: &str, buf: &mut String) {
        <&str as AttributeValue<R>>::to_html(self.as_ref(), key, buf);
    }

    fn to_template(_key: &str, _buf: &mut String) {}

    fn hydrate<const FROM_SERVER: bool>(
        self,
        key: &str,
        el: &R::Element,
    ) -> Self::State {
        let (el, _) = <&str as AttributeValue<R>>::hydrate::<FROM_SERVER>(
            self.as_ref(),
            key,
            el,
        );
        (el, self)
    }

    fn build(self, el: &R::Element, key: &str) -> Self::State {
        R::set_attribute(el, key, &self);
        (el.clone(), self)
    }

    fn rebuild(self, key: &str, state: &mut Self::State) {
        let (el, prev_value) = state;
        if self != *prev_value {
            R::set_attribute(el, key, &self);
        }
        *prev_value = self;
    }

    fn into_cloneable(self) -> Self::Cloneable {
        self
    }

    fn into_cloneable_owned(self) -> Self::CloneableOwned {
        self
    }
}
// TODO impl AttributeValue for Rc<str> and Arc<str> too

impl<R> AttributeValue<R> for bool
where
    R: Renderer,
{
    type State = (R::Element, bool);
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
        el: &R::Element,
    ) -> Self::State {
        // if we're actually hydrating from SSRed HTML, we don't need to set the attribute
        // if we're hydrating from a CSR-cloned <template>, we do need to set non-StaticAttr attributes
        if !FROM_SERVER {
            R::set_attribute(el, key, "");
        }
        (el.clone(), self)
    }

    fn build(self, el: &R::Element, key: &str) -> Self::State {
        if self {
            R::set_attribute(el, key, "");
        }
        (el.clone(), self)
    }

    fn rebuild(self, key: &str, state: &mut Self::State) {
        let (el, prev_value) = state;
        if self != *prev_value {
            if self {
                R::set_attribute(el, key, "");
            } else {
                R::remove_attribute(el, key);
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
}

impl<V, R> AttributeValue<R> for Option<V>
where
    V: AttributeValue<R>,
    R: Renderer,
{
    type State = (R::Element, Option<V::State>);
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
        el: &R::Element,
    ) -> Self::State {
        // if we're actually hydrating from SSRed HTML, we don't need to set the attribute
        // if we're hydrating from a CSR-cloned <template>, we do need to set non-StaticAttr attributes
        let state = if !FROM_SERVER {
            self.map(|v| v.hydrate::<FROM_SERVER>(key, el))
        } else {
            None
        };
        (el.clone(), state)
    }

    fn build(self, el: &R::Element, key: &str) -> Self::State {
        let el = el.clone();
        let v = self.map(|v| v.build(&el, key));
        (el, v)
    }

    fn rebuild(self, key: &str, state: &mut Self::State) {
        let (el, prev) = state;
        match (self, prev.as_mut()) {
            (None, None) => {}
            (None, Some(_)) => {
                R::remove_attribute(el, key);
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
}

fn escape_attr(value: &str) -> Cow<'_, str> {
    html_escape::encode_double_quoted_attribute(value)
}

macro_rules! render_primitive {
  ($($child_type:ty),* $(,)?) => {
      $(
        impl<R> AttributeValue<R> for $child_type
        where
            R: Renderer,
        {
            type State = (R::Element, $child_type);
            type Cloneable = Self;
            type CloneableOwned = Self;

            fn html_len(&self) -> usize {
                0
            }

            fn to_html(self, key: &str, buf: &mut String) {
                <String as AttributeValue<R>>::to_html(self.to_string(), key, buf);
            }

            fn to_template(_key: &str, _buf: &mut String) {}

            fn hydrate<const FROM_SERVER: bool>(
                self,
                key: &str,
                el: &R::Element,
            ) -> Self::State {
                // if we're actually hydrating from SSRed HTML, we don't need to set the attribute
                // if we're hydrating from a CSR-cloned <template>, we do need to set non-StaticAttr attributes
                if !FROM_SERVER {
                    R::set_attribute(el, key, &self.to_string());
                }
                (el.clone(), self)
            }

            fn build(self, el: &R::Element, key: &str) -> Self::State {
                R::set_attribute(el, key, &self.to_string());
                (el.to_owned(), self)
            }

            fn rebuild(self, key: &str, state: &mut Self::State) {
                let (el, prev_value) = state;
                if self != *prev_value {
                    R::set_attribute(el, key, &self.to_string());
                }
                *prev_value = self;
            }

            fn into_cloneable(self) -> Self::Cloneable {
                self
            }

            fn into_cloneable_owned(self) -> Self::CloneableOwned {
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
