use crate::renderer::Renderer;
use std::borrow::Cow;

pub trait AttributeValue<R: Renderer> {
    type State;

    fn to_html(self, key: &str, buf: &mut String);

    fn to_template(key: &str, buf: &mut String);

    fn hydrate<const FROM_SERVER: bool>(
        self,
        key: &str,
        el: &R::Element,
    ) -> Self::State;

    fn build(self, el: &R::Element, key: &str) -> Self::State;

    fn rebuild(self, key: &str, state: &mut Self::State);
}

impl<R> AttributeValue<R> for ()
where
    R: Renderer,
{
    type State = ();
    fn to_html(self, _key: &str, _buf: &mut String) {}

    fn to_template(_key: &str, _buf: &mut String) {}

    fn hydrate<const FROM_SERVER: bool>(self, _key: &str, _el: &R::Element) {}

    fn build(self, _el: &R::Element, _key: &str) -> Self::State {}

    fn rebuild(self, _key: &str, _state: &mut Self::State) {}
}

impl<'a, R> AttributeValue<R> for &'a str
where
    R: Renderer,
{
    type State = (R::Element, &'a str);

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
}

#[cfg(feature = "nightly")]
impl<R, const V: &'static str> AttributeValue<R>
    for crate::view::static_types::Static<V>
where
    R: Renderer,
{
    type State = ();

    fn to_html(self, key: &str, buf: &mut String) {
        <&str as AttributeValue<R>>::to_html(V, key, buf);
    }

    fn to_template(key: &str, buf: &mut String) {
        buf.push(' ');
        buf.push_str(key);
        buf.push_str("=\"");
        buf.push_str(&escape_attr(V));
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
}

impl<'a, R> AttributeValue<R> for &'a String
where
    R: Renderer,
{
    type State = (R::Element, &'a String);

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
}

impl<R> AttributeValue<R> for String
where
    R: Renderer,
{
    type State = (R::Element, String);

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
}

impl<R> AttributeValue<R> for bool
where
    R: Renderer,
{
    type State = (R::Element, bool);

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
}

impl<V, R> AttributeValue<R> for Option<V>
where
    V: AttributeValue<R>,
    R: Renderer,
{
    type State = (R::Element, Option<V::State>);

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
            (None, Some(_)) => R::remove_attribute(el, key),
            (Some(value), None) => {
                *prev = Some(value.build(el, key));
            }
            (Some(new), Some(old)) => new.rebuild(key, old),
        }
    }
}

// TODO
fn escape_attr(value: &str) -> Cow<'_, str> {
    value.into()
}
