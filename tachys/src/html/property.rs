use super::attribute::{Attribute, NextAttribute};
use crate::{
    renderer::DomRenderer,
    view::{Position, ToTemplate},
};
use send_wrapper::SendWrapper;
use std::{borrow::Cow, marker::PhantomData, sync::Arc};
use wasm_bindgen::JsValue;

#[inline(always)]
pub fn property<K, P, R>(key: K, value: P) -> Property<K, P, R>
where
    K: AsRef<str>,
    P: IntoProperty<R>,
    R: DomRenderer,
{
    Property {
        key,
        value: SendWrapper::new(value),
        rndr: PhantomData,
    }
}

#[derive(Debug)]
pub struct Property<K, P, R> {
    key: K,
    // property values will only be accessed in the browser
    value: SendWrapper<P>,
    rndr: PhantomData<R>,
}

impl<K, P, R> Clone for Property<K, P, R>
where
    K: Clone,
    P: Clone,
{
    fn clone(&self) -> Self {
        Self {
            key: self.key.clone(),
            value: self.value.clone(),
            rndr: PhantomData,
        }
    }
}

impl<K, P, R> Attribute<R> for Property<K, P, R>
where
    K: AsRef<str> + Send,
    P: IntoProperty<R>,
    R: DomRenderer,
{
    const MIN_LENGTH: usize = 0;

    type State = P::State;
    type Cloneable = Property<Arc<str>, P::Cloneable, R>;
    type CloneableOwned = Property<Arc<str>, P::CloneableOwned, R>;

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

    fn hydrate<const FROM_SERVER: bool>(self, el: &R::Element) -> Self::State {
        self.value
            .take()
            .hydrate::<FROM_SERVER>(el, self.key.as_ref())
    }

    fn build(self, el: &R::Element) -> Self::State {
        self.value.take().build(el, self.key.as_ref())
    }

    fn rebuild(self, state: &mut Self::State) {
        self.value.take().rebuild(state, self.key.as_ref())
    }

    fn into_cloneable(self) -> Self::Cloneable {
        Property {
            key: self.key.as_ref().into(),
            value: SendWrapper::new(self.value.take().into_cloneable()),
            rndr: self.rndr,
        }
    }

    fn into_cloneable_owned(self) -> Self::CloneableOwned {
        Property {
            key: self.key.as_ref().into(),
            value: SendWrapper::new(self.value.take().into_cloneable_owned()),
            rndr: self.rndr,
        }
    }
}

impl<K, P, R> NextAttribute<R> for Property<K, P, R>
where
    K: AsRef<str> + Send,
    P: IntoProperty<R>,
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

impl<K, P, R> ToTemplate for Property<K, P, R>
where
    K: AsRef<str>,
    P: IntoProperty<R>,
    R: DomRenderer,
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

pub trait IntoProperty<R: DomRenderer> {
    type State;
    type Cloneable: IntoProperty<R> + Clone;
    type CloneableOwned: IntoProperty<R> + Clone + 'static;

    fn hydrate<const FROM_SERVER: bool>(
        self,
        el: &R::Element,
        key: &str,
    ) -> Self::State;

    fn build(self, el: &R::Element, key: &str) -> Self::State;

    fn rebuild(self, state: &mut Self::State, key: &str);

    fn into_cloneable(self) -> Self::Cloneable;

    fn into_cloneable_owned(self) -> Self::CloneableOwned;
}

macro_rules! prop_type {
    ($prop_type:ty) => {
        impl<R> IntoProperty<R> for $prop_type
        where
            R: DomRenderer,
        {
            type State = (R::Element, JsValue);
            type Cloneable = Self;
            type CloneableOwned = Self;

            fn hydrate<const FROM_SERVER: bool>(
                self,
                el: &R::Element,
                key: &str,
            ) -> Self::State {
                let value = self.into();
                R::set_property(el, key, &value);
                (el.clone(), value)
            }

            fn build(self, el: &R::Element, key: &str) -> Self::State {
                let value = self.into();
                R::set_property(el, key, &value);
                (el.clone(), value)
            }

            fn rebuild(self, state: &mut Self::State, key: &str) {
                let (el, prev) = state;
                let value = self.into();
                if value != *prev {
                    R::set_property(el, key, &value);
                }
                *prev = value;
            }

            fn into_cloneable(self) -> Self::Cloneable {
                self
            }

            fn into_cloneable_owned(self) -> Self::CloneableOwned {
                self
            }
        }

        impl<R> IntoProperty<R> for Option<$prop_type>
        where
            R: DomRenderer,
        {
            type State = (R::Element, JsValue);
            type Cloneable = Self;
            type CloneableOwned = Self;

            fn hydrate<const FROM_SERVER: bool>(
                self,
                el: &R::Element,
                key: &str,
            ) -> Self::State {
                let was_some = self.is_some();
                let value = self.into();
                if was_some {
                    R::set_property(el, key, &value);
                }
                (el.clone(), value)
            }

            fn build(self, el: &R::Element, key: &str) -> Self::State {
                let was_some = self.is_some();
                let value = self.into();
                if was_some {
                    R::set_property(el, key, &value);
                }
                (el.clone(), value)
            }

            fn rebuild(self, state: &mut Self::State, key: &str) {
                let (el, prev) = state;
                let value = self.into();
                if value != *prev {
                    R::set_property(el, key, &value);
                }
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
        impl<R> IntoProperty<R> for $prop_type
        where
            R: DomRenderer,
        {
            type State = (R::Element, JsValue);
            type Cloneable = Arc<str>;
            type CloneableOwned = Arc<str>;

            fn hydrate<const FROM_SERVER: bool>(
                self,
                el: &R::Element,
                key: &str,
            ) -> Self::State {
                let value = JsValue::from(&*self);
                R::set_property(el, key, &value);
                (el.clone(), value)
            }

            fn build(self, el: &R::Element, key: &str) -> Self::State {
                let value = JsValue::from(&*self);
                R::set_property(el, key, &value);
                (el.clone(), value)
            }

            fn rebuild(self, state: &mut Self::State, key: &str) {
                let (el, prev) = state;
                let value = JsValue::from(&*self);
                if value != *prev {
                    R::set_property(el, key, &value);
                }
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

        impl<R> IntoProperty<R> for Option<$prop_type>
        where
            R: DomRenderer,
        {
            type State = (R::Element, JsValue);
            type Cloneable = Option<Arc<str>>;
            type CloneableOwned = Option<Arc<str>>;

            fn hydrate<const FROM_SERVER: bool>(
                self,
                el: &R::Element,
                key: &str,
            ) -> Self::State {
                let was_some = self.is_some();
                let value = JsValue::from(self.map(|n| JsValue::from_str(&n)));
                if was_some {
                    R::set_property(el, key, &value);
                }
                (el.clone(), value)
            }

            fn build(self, el: &R::Element, key: &str) -> Self::State {
                let was_some = self.is_some();
                let value = JsValue::from(self.map(|n| JsValue::from_str(&n)));
                if was_some {
                    R::set_property(el, key, &value);
                }
                (el.clone(), value)
            }

            fn rebuild(self, state: &mut Self::State, key: &str) {
                let (el, prev) = state;
                let value = JsValue::from(self.map(|n| JsValue::from_str(&n)));
                if value != *prev {
                    R::set_property(el, key, &value);
                }
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

impl<R> IntoProperty<R> for Arc<str>
where
    R: DomRenderer,
{
    type State = (R::Element, JsValue);
    type Cloneable = Self;
    type CloneableOwned = Self;

    fn hydrate<const FROM_SERVER: bool>(
        self,
        el: &R::Element,
        key: &str,
    ) -> Self::State {
        let value = JsValue::from_str(self.as_ref());
        R::set_property(el, key, &value);
        (el.clone(), value)
    }

    fn build(self, el: &R::Element, key: &str) -> Self::State {
        let value = JsValue::from_str(self.as_ref());
        R::set_property(el, key, &value);
        (el.clone(), value)
    }

    fn rebuild(self, state: &mut Self::State, key: &str) {
        let (el, prev) = state;
        let value = JsValue::from_str(self.as_ref());
        if value != *prev {
            R::set_property(el, key, &value);
        }
        *prev = value;
    }

    fn into_cloneable(self) -> Self::Cloneable {
        self
    }

    fn into_cloneable_owned(self) -> Self::CloneableOwned {
        self
    }
}

impl<R> IntoProperty<R> for Option<Arc<str>>
where
    R: DomRenderer,
{
    type State = (R::Element, JsValue);
    type Cloneable = Self;
    type CloneableOwned = Self;

    fn hydrate<const FROM_SERVER: bool>(
        self,
        el: &R::Element,
        key: &str,
    ) -> Self::State {
        let was_some = self.is_some();
        let value = JsValue::from(self.map(|n| JsValue::from_str(&n)));
        if was_some {
            R::set_property(el, key, &value);
        }
        (el.clone(), value)
    }

    fn build(self, el: &R::Element, key: &str) -> Self::State {
        let was_some = self.is_some();
        let value = JsValue::from(self.map(|n| JsValue::from_str(&n)));
        if was_some {
            R::set_property(el, key, &value);
        }
        (el.clone(), value)
    }

    fn rebuild(self, state: &mut Self::State, key: &str) {
        let (el, prev) = state;
        let value = JsValue::from(self.map(|n| JsValue::from_str(&n)));
        if value != *prev {
            R::set_property(el, key, &value);
        }
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
