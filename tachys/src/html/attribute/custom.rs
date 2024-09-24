use super::NextAttribute;
use crate::{
    html::attribute::{Attribute, AttributeValue},
    view::{add_attr::AddAnyAttr, Position, ToTemplate},
};
use std::{borrow::Cow, sync::Arc};

/// Adds a custom attribute with any key-value combintion.
#[inline(always)]
pub fn custom_attribute<K, V>(key: K, value: V) -> CustomAttr<K, V>
where
    K: CustomAttributeKey,
    V: AttributeValue,
{
    CustomAttr { key, value }
}

/// A custom attribute with any key-value combination.
#[derive(Debug)]
pub struct CustomAttr<K, V>
where
    K: CustomAttributeKey,
    V: AttributeValue,
{
    key: K,
    value: V,
}

impl<K, V> Clone for CustomAttr<K, V>
where
    K: CustomAttributeKey,
    V: AttributeValue + Clone,
{
    fn clone(&self) -> Self {
        Self {
            key: self.key.clone(),
            value: self.value.clone(),
        }
    }
}

impl<K, V> Attribute for CustomAttr<K, V>
where
    K: CustomAttributeKey,
    V: AttributeValue,
{
    const MIN_LENGTH: usize = 0;
    type AsyncOutput = CustomAttr<K, V::AsyncOutput>;
    type State = V::State;
    type Cloneable = CustomAttr<K, V::Cloneable>;
    type CloneableOwned = CustomAttr<K, V::CloneableOwned>;

    fn html_len(&self) -> usize {
        self.key.as_ref().len() + 3 + self.value.html_len()
    }

    fn to_html(
        self,
        buf: &mut String,
        _class: &mut String,
        _style: &mut String,
        _inner_html: &mut String,
    ) {
        self.value.to_html(self.key.as_ref(), buf);
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        el: &crate::renderer::types::Element,
    ) -> Self::State {
        if !K::KEY.is_empty() {
            self.value.hydrate::<FROM_SERVER>(self.key.as_ref(), el)
        } else {
            self.value.build(el, self.key.as_ref())
        }
    }

    fn build(self, el: &crate::renderer::types::Element) -> Self::State {
        self.value.build(el, self.key.as_ref())
    }

    fn rebuild(self, state: &mut Self::State) {
        self.value.rebuild(self.key.as_ref(), state);
    }

    fn into_cloneable(self) -> Self::Cloneable {
        CustomAttr {
            key: self.key,
            value: self.value.into_cloneable(),
        }
    }

    fn into_cloneable_owned(self) -> Self::CloneableOwned {
        CustomAttr {
            key: self.key,
            value: self.value.into_cloneable_owned(),
        }
    }

    fn dry_resolve(&mut self) {
        self.value.dry_resolve();
    }

    async fn resolve(self) -> Self::AsyncOutput {
        CustomAttr {
            key: self.key,
            value: self.value.resolve().await,
        }
    }
}

impl<K, V> NextAttribute for CustomAttr<K, V>
where
    K: CustomAttributeKey,
    V: AttributeValue,
{
    type Output<NewAttr: Attribute> = (Self, NewAttr);

    fn add_any_attr<NewAttr: Attribute>(
        self,
        new_attr: NewAttr,
    ) -> Self::Output<NewAttr> {
        (self, new_attr)
    }
}

impl<K, V> ToTemplate for CustomAttr<K, V>
where
    K: CustomAttributeKey,
    V: AttributeValue,
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

// TODO this needs to be a method, not a const
/// Defines a custom attribute key.
pub trait CustomAttributeKey: Clone + AsRef<str> + Send + 'static {
    /// The attribute name.
    const KEY: &'static str;
}

impl CustomAttributeKey for &'static str {
    const KEY: &'static str = "";
}

impl CustomAttributeKey for Cow<'static, str> {
    const KEY: &'static str = "";
}

impl CustomAttributeKey for String {
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

/// Adds a custom attribute to an element.
pub trait CustomAttribute<K, V>
where
    K: CustomAttributeKey,
    V: AttributeValue,

    Self: Sized + AddAnyAttr,
{
    /// Adds an HTML attribute by key and value.
    fn attr(
        self,
        key: K,
        value: V,
    ) -> <Self as AddAnyAttr>::Output<CustomAttr<K, V>> {
        self.add_any_attr(custom_attribute(key, value))
    }
}

impl<T, K, V> CustomAttribute<K, V> for T
where
    T: AddAnyAttr,
    K: CustomAttributeKey,
    V: AttributeValue,
{
}
