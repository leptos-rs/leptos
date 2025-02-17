use super::{ElementWithChildren, HtmlElement};
use crate::{
    html::attribute::{
        maybe_next_attr_erasure_macros::{
            next_attr_combine, next_attr_output_type,
        },
        Attribute, NextAttribute,
    },
    renderer::Rndr,
    view::add_attr::AddAnyAttr,
};
use std::{future::Future, sync::Arc};

/// Returns an [`Attribute`] that sets the inner HTML of an element.
///
/// No children should be given to this element, as this HTML will be used instead.
///
/// # Security
/// Be very careful when using this method. Always remember to
/// sanitize the input to avoid a cross-site scripting (XSS)
/// vulnerability.
#[inline(always)]
pub fn inner_html<T>(value: T) -> InnerHtml<T>
where
    T: InnerHtmlValue,
{
    InnerHtml { value }
}

/// Sets the inner HTML of an element.
#[derive(Debug)]
pub struct InnerHtml<T> {
    value: T,
}

impl<T> Clone for InnerHtml<T>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone(),
        }
    }
}

impl<T> Attribute for InnerHtml<T>
where
    T: InnerHtmlValue,
{
    const MIN_LENGTH: usize = 0;

    type AsyncOutput = InnerHtml<T::AsyncOutput>;
    type State = T::State;
    type Cloneable = InnerHtml<T::Cloneable>;
    type CloneableOwned = InnerHtml<T::CloneableOwned>;

    fn html_len(&self) -> usize {
        self.value.html_len()
    }

    fn to_html(
        self,
        _buf: &mut String,
        _class: &mut String,
        _style: &mut String,
        inner_html: &mut String,
    ) {
        self.value.to_html(inner_html);
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        el: &crate::renderer::types::Element,
    ) -> Self::State {
        self.value.hydrate::<FROM_SERVER>(el)
    }

    fn build(self, el: &crate::renderer::types::Element) -> Self::State {
        self.value.build(el)
    }

    fn rebuild(self, state: &mut Self::State) {
        self.value.rebuild(state);
    }

    fn into_cloneable(self) -> Self::Cloneable {
        InnerHtml {
            value: self.value.into_cloneable(),
        }
    }

    fn into_cloneable_owned(self) -> Self::CloneableOwned {
        InnerHtml {
            value: self.value.into_cloneable_owned(),
        }
    }

    fn dry_resolve(&mut self) {
        self.value.dry_resolve();
    }

    async fn resolve(self) -> Self::AsyncOutput {
        InnerHtml {
            value: self.value.resolve().await,
        }
    }
}

impl<T> NextAttribute for InnerHtml<T>
where
    T: InnerHtmlValue,
{
    next_attr_output_type!(Self, NewAttr);

    fn add_any_attr<NewAttr: Attribute>(
        self,
        new_attr: NewAttr,
    ) -> Self::Output<NewAttr> {
        next_attr_combine!(self, new_attr)
    }
}

/// Sets the inner HTML of an element.
pub trait InnerHtmlAttribute<T>
where
    T: InnerHtmlValue,

    Self: Sized + AddAnyAttr,
{
    /// Sets the inner HTML of this element.
    ///
    /// No children should be given to this element, as this HTML will be used instead.
    ///
    /// # Security
    /// Be very careful when using this method. Always remember to
    /// sanitize the input to avoid a cross-site scripting (XSS)
    /// vulnerability.
    fn inner_html(
        self,
        value: T,
    ) -> <Self as AddAnyAttr>::Output<InnerHtml<T>> {
        self.add_any_attr(inner_html(value))
    }
}

impl<T, E, At> InnerHtmlAttribute<T> for HtmlElement<E, At, ()>
where
    Self: AddAnyAttr,
    E: ElementWithChildren,
    At: Attribute,
    T: InnerHtmlValue,
{
    fn inner_html(
        self,
        value: T,
    ) -> <Self as AddAnyAttr>::Output<InnerHtml<T>> {
        self.add_any_attr(inner_html(value))
    }
}

/// A possible value for [`InnerHtml`].
pub trait InnerHtmlValue: Send {
    /// The type after all async data have resolved.
    type AsyncOutput: InnerHtmlValue;
    /// The view state retained between building and rebuilding.
    type State;
    /// An equivalent value that can be cloned.
    type Cloneable: InnerHtmlValue + Clone;
    /// An equivalent value that can be cloned and is `'static`.
    type CloneableOwned: InnerHtmlValue + Clone + 'static;

    /// The estimated length of the HTML.
    fn html_len(&self) -> usize;

    /// Renders the class to HTML.
    fn to_html(self, buf: &mut String);

    /// Renders the class to HTML for a `<template>`.
    fn to_template(buf: &mut String);

    /// Adds interactivity as necessary, given DOM nodes that were created from HTML that has
    /// either been rendered on the server, or cloned for a `<template>`.
    fn hydrate<const FROM_SERVER: bool>(
        self,
        el: &crate::renderer::types::Element,
    ) -> Self::State;

    /// Adds this class to the element during client-side rendering.
    fn build(self, el: &crate::renderer::types::Element) -> Self::State;

    /// Updates the value.
    fn rebuild(self, state: &mut Self::State);

    /// Converts this to a cloneable type.
    fn into_cloneable(self) -> Self::Cloneable;

    /// Converts this to a cloneable, owned type.
    fn into_cloneable_owned(self) -> Self::CloneableOwned;

    /// “Runs” the attribute without other side effects. For primitive types, this is a no-op. For
    /// reactive types, this can be used to gather data about reactivity or about asynchronous data
    /// that needs to be loaded.
    fn dry_resolve(&mut self);

    /// “Resolves” this into a type that is not waiting for any asynchronous data.
    fn resolve(self) -> impl Future<Output = Self::AsyncOutput> + Send;
}

impl InnerHtmlValue for String {
    type AsyncOutput = Self;
    type State = (crate::renderer::types::Element, Self);
    type Cloneable = Arc<str>;
    type CloneableOwned = Arc<str>;

    fn html_len(&self) -> usize {
        self.len()
    }

    fn to_html(self, buf: &mut String) {
        buf.push_str(&self);
    }

    fn to_template(_buf: &mut String) {}

    fn hydrate<const FROM_SERVER: bool>(
        self,
        el: &crate::renderer::types::Element,
    ) -> Self::State {
        if !FROM_SERVER {
            Rndr::set_inner_html(el, &self);
        }
        (el.clone(), self)
    }

    fn build(self, el: &crate::renderer::types::Element) -> Self::State {
        Rndr::set_inner_html(el, &self);
        (el.clone(), self)
    }

    fn rebuild(self, state: &mut Self::State) {
        if self != state.1 {
            Rndr::set_inner_html(&state.0, &self);
            state.1 = self;
        }
    }

    fn into_cloneable(self) -> Self::Cloneable {
        self.into()
    }

    fn into_cloneable_owned(self) -> Self::Cloneable {
        self.into()
    }

    fn dry_resolve(&mut self) {}

    async fn resolve(self) -> Self::AsyncOutput {
        self
    }
}

impl InnerHtmlValue for Arc<str> {
    type AsyncOutput = Self;
    type State = (crate::renderer::types::Element, Self);
    type Cloneable = Self;
    type CloneableOwned = Self;

    fn html_len(&self) -> usize {
        self.len()
    }

    fn to_html(self, buf: &mut String) {
        buf.push_str(&self);
    }

    fn to_template(_buf: &mut String) {}

    fn hydrate<const FROM_SERVER: bool>(
        self,
        el: &crate::renderer::types::Element,
    ) -> Self::State {
        if !FROM_SERVER {
            Rndr::set_inner_html(el, &self);
        }
        (el.clone(), self)
    }

    fn build(self, el: &crate::renderer::types::Element) -> Self::State {
        Rndr::set_inner_html(el, &self);
        (el.clone(), self)
    }

    fn rebuild(self, state: &mut Self::State) {
        if !Arc::ptr_eq(&self, &state.1) {
            Rndr::set_inner_html(&state.0, &self);
            state.1 = self;
        }
    }

    fn into_cloneable(self) -> Self::Cloneable {
        self
    }

    fn into_cloneable_owned(self) -> Self::Cloneable {
        self
    }

    fn dry_resolve(&mut self) {}

    async fn resolve(self) -> Self::AsyncOutput {
        self
    }
}

impl InnerHtmlValue for &str {
    type AsyncOutput = Self;
    type State = (crate::renderer::types::Element, Self);
    type Cloneable = Self;
    type CloneableOwned = Arc<str>;

    fn html_len(&self) -> usize {
        self.len()
    }

    fn to_html(self, buf: &mut String) {
        buf.push_str(self);
    }

    fn to_template(_buf: &mut String) {}

    fn hydrate<const FROM_SERVER: bool>(
        self,
        el: &crate::renderer::types::Element,
    ) -> Self::State {
        if !FROM_SERVER {
            Rndr::set_inner_html(el, self);
        }
        (el.clone(), self)
    }

    fn build(self, el: &crate::renderer::types::Element) -> Self::State {
        Rndr::set_inner_html(el, self);
        (el.clone(), self)
    }

    fn rebuild(self, state: &mut Self::State) {
        if self != state.1 {
            Rndr::set_inner_html(&state.0, self);
            state.1 = self;
        }
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

impl<T> InnerHtmlValue for Option<T>
where
    T: InnerHtmlValue,
{
    type AsyncOutput = Self;
    type State = (crate::renderer::types::Element, Option<T::State>);
    type Cloneable = Option<T::Cloneable>;
    type CloneableOwned = Option<T::CloneableOwned>;

    fn html_len(&self) -> usize {
        match self {
            Some(i) => i.html_len(),
            None => 0,
        }
    }

    fn to_html(self, buf: &mut String) {
        if let Some(value) = self {
            value.to_html(buf);
        }
    }

    fn to_template(_buf: &mut String) {}

    fn hydrate<const FROM_SERVER: bool>(
        self,
        el: &crate::renderer::types::Element,
    ) -> Self::State {
        (el.clone(), self.map(|n| n.hydrate::<FROM_SERVER>(el)))
    }

    fn build(self, el: &crate::renderer::types::Element) -> Self::State {
        (el.clone(), self.map(|n| n.build(el)))
    }

    fn rebuild(self, state: &mut Self::State) {
        let new_state = match (self, &mut state.1) {
            (None, None) => None,
            (None, Some(_)) => {
                Rndr::set_inner_html(&state.0, "");
                Some(None)
            }
            (Some(new), None) => Some(Some(new.build(&state.0))),
            (Some(new), Some(state)) => {
                new.rebuild(state);
                None
            }
        };
        if let Some(new_state) = new_state {
            state.1 = new_state;
        }
    }

    fn into_cloneable(self) -> Self::Cloneable {
        self.map(|inner| inner.into_cloneable())
    }

    fn into_cloneable_owned(self) -> Self::CloneableOwned {
        self.map(|inner| inner.into_cloneable_owned())
    }

    fn dry_resolve(&mut self) {}

    async fn resolve(self) -> Self::AsyncOutput {
        self
    }
}
