#![allow(deprecated)]

use crate::TextProp;
use std::rc::Rc;

/// A collection of additional HTML attributes to be applied to an element,
/// each of which may or may not be reactive.
#[derive(Clone)]
#[repr(transparent)]
#[deprecated = "Most uses of `AdditionalAttributes` can be replaced with `#[prop(attrs)]` \
and the `attr:` syntax. If you have a use case that still requires `AdditionalAttributes`, please \
open a GitHub issue here and share it: https://github.com/leptos-rs/leptos"]
pub struct AdditionalAttributes(pub(crate) Rc<[(String, TextProp)]>);

impl<I, T, U> From<I> for AdditionalAttributes
where
    I: IntoIterator<Item = (T, U)>,
    T: Into<String>,
    U: Into<TextProp>,
{
    fn from(value: I) -> Self {
        Self(
            value
                .into_iter()
                .map(|(k, v)| (k.into(), v.into()))
                .collect(),
        )
    }
}

impl Default for AdditionalAttributes {
    fn default() -> Self {
        Self([].into_iter().collect())
    }
}

/// Iterator over additional HTML attributes.
#[repr(transparent)]
pub struct AdditionalAttributesIter<'a>(
    std::slice::Iter<'a, (String, TextProp)>,
);

impl<'a> Iterator for AdditionalAttributesIter<'a> {
    type Item = &'a (String, TextProp);

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}

impl<'a> IntoIterator for &'a AdditionalAttributes {
    type Item = &'a (String, TextProp);
    type IntoIter = AdditionalAttributesIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        AdditionalAttributesIter(self.0.iter())
    }
}
