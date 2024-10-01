use super::attribute::{Attribute, NextAttribute};
use crate::{
    renderer::Rndr,
    view::{Position, ToTemplate},
};
use send_wrapper::SendWrapper;
use std::{borrow::Cow, sync::Arc};
use wasm_bindgen::JsValue;

/// Creates an [`Attribute`] that will set a DOM property on an element.
#[inline(always)]
pub fn prop<K, P>(key: K, value: P) -> Property<K, P>
where
    K: AsRef<str>,
    P: IntoProperty,
{
    Property {
        key,
        value: Some(SendWrapper::new(value)),
    }
}

/// An [`Attribute`] that will set a DOM property on an element.
#[derive(Debug)]
pub struct Property<K, P> {
    key: K,
    // property values will only be accessed in the browser
    value: Option<SendWrapper<P>>,
}

impl<K, P> Clone for Property<K, P>
where
    K: Clone,
    P: Clone,
{
    fn clone(&self) -> Self {
        Self {
            key: self.key.clone(),
            value: self.value.clone(),
        }
    }
}

impl<K, P> Attribute for Property<K, P>
where
    K: AsRef<str> + Send,
    P: IntoProperty,
{
    const MIN_LENGTH: usize = 0;

    type AsyncOutput = Self;
    type State = P::State;
    type Cloneable = Property<Arc<str>, P::Cloneable>;
    type CloneableOwned = Property<Arc<str>, P::CloneableOwned>;

    #[inline(always)]
    fn html_len(&self) -> usize {
        0
    }

    fn to_html(
        self,
        _buf: &mut String,
        _class: &mut String,
        _style: &mut String,
        _inner_html: &mut String,
    ) {
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        el: &crate::renderer::types::Element,
    ) -> Self::State {
        self.value
            .expect("property removed early")
            .take()
            .hydrate::<FROM_SERVER>(el, self.key.as_ref())
    }

    fn build(self, el: &crate::renderer::types::Element) -> Self::State {
        self.value
            .expect("property removed early")
            .take()
            .build(el, self.key.as_ref())
    }

    fn rebuild(self, state: &mut Self::State) {
        self.value
            .expect("property removed early")
            .take()
            .rebuild(state, self.key.as_ref())
    }

    fn into_cloneable(self) -> Self::Cloneable {
        Property {
            key: self.key.as_ref().into(),
            value: self
                .value
                .map(|value| SendWrapper::new(value.take().into_cloneable())),
        }
    }

    fn into_cloneable_owned(self) -> Self::CloneableOwned {
        Property {
            key: self.key.as_ref().into(),
            value: self.value.map(|value| {
                SendWrapper::new(value.take().into_cloneable_owned())
            }),
        }
    }

    fn dry_resolve(&mut self) {
        // dry_resolve() only runs during SSR, and we should use it to
        // synchronously remove and drop the SendWrapper value
        // we don't need this value during SSR and leaving it here could drop it
        // from a different thread
        self.value.take();
    }

    async fn resolve(self) -> Self::AsyncOutput {
        self
    }
}

impl<K, P> NextAttribute for Property<K, P>
where
    K: AsRef<str> + Send,
    P: IntoProperty,
{
    type Output<NewAttr: Attribute> = (Self, NewAttr);

    fn add_any_attr<NewAttr: Attribute>(
        self,
        new_attr: NewAttr,
    ) -> Self::Output<NewAttr> {
        (self, new_attr)
    }
}

impl<K, P> ToTemplate for Property<K, P>
where
    K: AsRef<str>,
    P: IntoProperty,
{
    fn to_template(
        _buf: &mut String,
        _class: &mut String,
        _style: &mut String,
        _inner_html: &mut String,
        _position: &mut Position,
    ) {
    }
}

/// A possible value for a DOM property.
pub trait IntoProperty {
    /// The view state retained between building and rebuilding.
    type State;
    /// An equivalent value that can be cloned.
    type Cloneable: IntoProperty + Clone;
    /// An equivalent value that can be cloned and is `'static`.
    type CloneableOwned: IntoProperty + Clone + 'static;

    /// Adds the property on an element created from HTML.
    fn hydrate<const FROM_SERVER: bool>(
        self,
        el: &crate::renderer::types::Element,
        key: &str,
    ) -> Self::State;

    /// Adds the property during client-side rendering.
    fn build(
        self,
        el: &crate::renderer::types::Element,
        key: &str,
    ) -> Self::State;

    /// Updates the property with a new value.
    fn rebuild(self, state: &mut Self::State, key: &str);

    /// Converts this to a cloneable type.
    fn into_cloneable(self) -> Self::Cloneable;

    /// Converts this to a cloneable, owned type.
    fn into_cloneable_owned(self) -> Self::CloneableOwned;
}

macro_rules! prop_type {
    ($prop_type:ty) => {
        impl IntoProperty for $prop_type {
            type State = (crate::renderer::types::Element, JsValue);
            type Cloneable = Self;
            type CloneableOwned = Self;

            fn hydrate<const FROM_SERVER: bool>(
                self,
                el: &crate::renderer::types::Element,
                key: &str,
            ) -> Self::State {
                let value = self.into();
                Rndr::set_property(el, key, &value);
                (el.clone(), value)
            }

            fn build(
                self,
                el: &crate::renderer::types::Element,
                key: &str,
            ) -> Self::State {
                let value = self.into();
                Rndr::set_property(el, key, &value);
                (el.clone(), value)
            }

            fn rebuild(self, state: &mut Self::State, key: &str) {
                let (el, prev) = state;
                let value = self.into();
                Rndr::set_property(el, key, &value);
                *prev = value;
            }

            fn into_cloneable(self) -> Self::Cloneable {
                self
            }

            fn into_cloneable_owned(self) -> Self::CloneableOwned {
                self
            }
        }

        impl IntoProperty for Option<$prop_type> {
            type State = (crate::renderer::types::Element, JsValue);
            type Cloneable = Self;
            type CloneableOwned = Self;

            fn hydrate<const FROM_SERVER: bool>(
                self,
                el: &crate::renderer::types::Element,
                key: &str,
            ) -> Self::State {
                let was_some = self.is_some();
                let value = self.into();
                if was_some {
                    Rndr::set_property(el, key, &value);
                }
                (el.clone(), value)
            }

            fn build(
                self,
                el: &crate::renderer::types::Element,
                key: &str,
            ) -> Self::State {
                let was_some = self.is_some();
                let value = self.into();
                if was_some {
                    Rndr::set_property(el, key, &value);
                }
                (el.clone(), value)
            }

            fn rebuild(self, state: &mut Self::State, key: &str) {
                let (el, prev) = state;
                let value = self.into();
                Rndr::set_property(el, key, &value);
                *prev = value;
            }

            fn into_cloneable(self) -> Self::Cloneable {
                self
            }

            fn into_cloneable_owned(self) -> Self::CloneableOwned {
                self
            }
        }
    };
}

macro_rules! prop_type_str {
    ($prop_type:ty) => {
        impl IntoProperty for $prop_type {
            type State = (crate::renderer::types::Element, JsValue);
            type Cloneable = Arc<str>;
            type CloneableOwned = Arc<str>;

            fn hydrate<const FROM_SERVER: bool>(
                self,
                el: &crate::renderer::types::Element,
                key: &str,
            ) -> Self::State {
                let value = JsValue::from(&*self);
                Rndr::set_property(el, key, &value);
                (el.clone(), value)
            }

            fn build(
                self,
                el: &crate::renderer::types::Element,
                key: &str,
            ) -> Self::State {
                let value = JsValue::from(&*self);
                Rndr::set_property(el, key, &value);
                (el.clone(), value)
            }

            fn rebuild(self, state: &mut Self::State, key: &str) {
                let (el, prev) = state;
                let value = JsValue::from(&*self);
                Rndr::set_property(el, key, &value);
                *prev = value;
            }

            fn into_cloneable(self) -> Self::Cloneable {
                let this: &str = &*self;
                this.into()
            }

            fn into_cloneable_owned(self) -> Self::CloneableOwned {
                let this: &str = &*self;
                this.into()
            }
        }

        impl IntoProperty for Option<$prop_type> {
            type State = (crate::renderer::types::Element, JsValue);
            type Cloneable = Option<Arc<str>>;
            type CloneableOwned = Option<Arc<str>>;

            fn hydrate<const FROM_SERVER: bool>(
                self,
                el: &crate::renderer::types::Element,
                key: &str,
            ) -> Self::State {
                let was_some = self.is_some();
                let value = JsValue::from(self.map(|n| JsValue::from_str(&n)));
                if was_some {
                    Rndr::set_property(el, key, &value);
                }
                (el.clone(), value)
            }

            fn build(
                self,
                el: &crate::renderer::types::Element,
                key: &str,
            ) -> Self::State {
                let was_some = self.is_some();
                let value = JsValue::from(self.map(|n| JsValue::from_str(&n)));
                if was_some {
                    Rndr::set_property(el, key, &value);
                }
                (el.clone(), value)
            }

            fn rebuild(self, state: &mut Self::State, key: &str) {
                let (el, prev) = state;
                let value = JsValue::from(self.map(|n| JsValue::from_str(&n)));
                Rndr::set_property(el, key, &value);
                *prev = value;
            }

            fn into_cloneable(self) -> Self::Cloneable {
                self.map(|n| {
                    let this: &str = &*n;
                    this.into()
                })
            }

            fn into_cloneable_owned(self) -> Self::CloneableOwned {
                self.map(|n| {
                    let this: &str = &*n;
                    this.into()
                })
            }
        }
    };
}

impl IntoProperty for Arc<str> {
    type State = (crate::renderer::types::Element, JsValue);
    type Cloneable = Self;
    type CloneableOwned = Self;

    fn hydrate<const FROM_SERVER: bool>(
        self,
        el: &crate::renderer::types::Element,
        key: &str,
    ) -> Self::State {
        let value = JsValue::from_str(self.as_ref());
        Rndr::set_property(el, key, &value);
        (el.clone(), value)
    }

    fn build(
        self,
        el: &crate::renderer::types::Element,
        key: &str,
    ) -> Self::State {
        let value = JsValue::from_str(self.as_ref());
        Rndr::set_property(el, key, &value);
        (el.clone(), value)
    }

    fn rebuild(self, state: &mut Self::State, key: &str) {
        let (el, prev) = state;
        let value = JsValue::from_str(self.as_ref());
        Rndr::set_property(el, key, &value);
        *prev = value;
    }

    fn into_cloneable(self) -> Self::Cloneable {
        self
    }

    fn into_cloneable_owned(self) -> Self::CloneableOwned {
        self
    }
}

impl IntoProperty for Option<Arc<str>> {
    type State = (crate::renderer::types::Element, JsValue);
    type Cloneable = Self;
    type CloneableOwned = Self;

    fn hydrate<const FROM_SERVER: bool>(
        self,
        el: &crate::renderer::types::Element,
        key: &str,
    ) -> Self::State {
        let was_some = self.is_some();
        let value = JsValue::from(self.map(|n| JsValue::from_str(&n)));
        if was_some {
            Rndr::set_property(el, key, &value);
        }
        (el.clone(), value)
    }

    fn build(
        self,
        el: &crate::renderer::types::Element,
        key: &str,
    ) -> Self::State {
        let was_some = self.is_some();
        let value = JsValue::from(self.map(|n| JsValue::from_str(&n)));
        if was_some {
            Rndr::set_property(el, key, &value);
        }
        (el.clone(), value)
    }

    fn rebuild(self, state: &mut Self::State, key: &str) {
        let (el, prev) = state;
        let value = JsValue::from(self.map(|n| JsValue::from_str(&n)));
        Rndr::set_property(el, key, &value);
        *prev = value;
    }

    fn into_cloneable(self) -> Self::Cloneable {
        self
    }

    fn into_cloneable_owned(self) -> Self::CloneableOwned {
        self
    }
}

prop_type!(JsValue);
prop_type!(usize);
prop_type!(u8);
prop_type!(u16);
prop_type!(u32);
prop_type!(u64);
prop_type!(u128);
prop_type!(isize);
prop_type!(i8);
prop_type!(i16);
prop_type!(i32);
prop_type!(i64);
prop_type!(i128);
prop_type!(f32);
prop_type!(f64);
prop_type!(bool);

prop_type_str!(String);
prop_type_str!(&String);
prop_type_str!(&str);
prop_type_str!(Cow<'_, str>);
