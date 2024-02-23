use super::NextAttribute;
use crate::{
    html::attribute::{Attribute, AttributeValue},
    renderer::DomRenderer,
    view::{add_attr::AddAnyAttr, Position, ToTemplate},
};
use std::{borrow::Cow, marker::PhantomData, rc::Rc, sync::Arc};

#[inline(always)]
pub fn custom_attribute<K, V, R>(key: K, value: V) -> CustomAttr<K, V, R>
where
    K: CustomAttributeKey,
    V: AttributeValue<R>,
    R: DomRenderer,
{
    CustomAttr {
        key,
        value,
        rndr: PhantomData,
    }
}

pub struct CustomAttr<K, V, R>
where
    K: CustomAttributeKey,
    V: AttributeValue<R>,
    R: DomRenderer,
{
    key: K,
    value: V,
    rndr: PhantomData<R>,
}

impl<K, V, R> Attribute<R> for CustomAttr<K, V, R>
where
    K: CustomAttributeKey,
    V: AttributeValue<R>,
    R: DomRenderer,
{
    const MIN_LENGTH: usize = 0;
    type State = V::State;

    fn to_html(
        self,
        buf: &mut String,
        _class: &mut String,
        _style: &mut String,
        _inner_html: &mut String,
    ) {
        self.value.to_html(self.key.as_ref(), buf);
    }

    fn hydrate<const FROM_SERVER: bool>(self, el: &R::Element) -> Self::State {
        if !K::KEY.is_empty() {
            self.value.hydrate::<FROM_SERVER>(self.key.as_ref(), el)
        } else {
            self.value.build(el, self.key.as_ref())
        }
    }

    fn build(self, el: &R::Element) -> Self::State {
        self.value.build(el, self.key.as_ref())
    }

    fn rebuild(self, state: &mut Self::State) {
        self.value.rebuild(self.key.as_ref(), state);
    }
}

impl<K, V, R> NextAttribute<R> for CustomAttr<K, V, R>
where
    K: CustomAttributeKey,
    V: AttributeValue<R>,
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

impl<K, V, R> ToTemplate for CustomAttr<K, V, R>
where
    K: CustomAttributeKey,
    V: AttributeValue<R>,
    R: DomRenderer,
{
    fn to_template(
        buf: &mut String,
        _class: &mut String,
        _style: &mut String,
        _inner_html: &mut String,
        _position: &mut Position,
    ) {
        if !K::KEY.is_empty() {
            V::to_template(K::KEY, buf);
        }
    }
}

pub trait CustomAttributeKey: AsRef<str> {
    const KEY: &'static str;
}

impl<'a> CustomAttributeKey for &'a str {
    const KEY: &'static str = "";
}

impl<'a> CustomAttributeKey for Cow<'a, str> {
    const KEY: &'static str = "";
}

impl CustomAttributeKey for &String {
    const KEY: &'static str = "";
}

impl CustomAttributeKey for String {
    const KEY: &'static str = "";
}

impl CustomAttributeKey for Rc<str> {
    const KEY: &'static str = "";
}

impl CustomAttributeKey for Arc<str> {
    const KEY: &'static str = "";
}

#[cfg(feature = "nightly")]
impl<const K: &'static str> CustomAttributeKey
    for crate::view::static_types::Static<K>
{
    const KEY: &'static str = K;
}

pub trait CustomAttribute<K, V, Rndr>
where
    K: CustomAttributeKey,
    V: AttributeValue<Rndr>,
    Rndr: DomRenderer,
    Self: Sized + AddAnyAttr<Rndr>,
{
    fn attr(
        self,
        key: K,
        value: V,
    ) -> <Self as AddAnyAttr<Rndr>>::Output<CustomAttr<K, V, Rndr>> {
        self.add_any_attr(custom_attribute(key, value))
    }
}

impl<T, K, V, Rndr> CustomAttribute<K, V, Rndr> for T
where
    T: AddAnyAttr<Rndr>,
    K: CustomAttributeKey,
    V: AttributeValue<Rndr>,
    Rndr: DomRenderer,
{
}
