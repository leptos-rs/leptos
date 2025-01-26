use super::{
    attribute::{
        maybe_next_attr_erasure_macros::next_attr_output_type,
        panic_on_clone_attribute::PanicOnCloneAttr, Attribute, NextAttribute,
    },
    element::ElementType,
};
use crate::{
    html::{
        attribute::maybe_next_attr_erasure_macros::next_attr_combine,
        element::HtmlElement,
    },
    prelude::Render,
    view::add_attr::AddAnyAttr,
};
use std::marker::PhantomData;

/// Describes a container that can be used to hold a reference to an HTML element.
pub trait NodeRefContainer<E>: Send + Clone + 'static
where
    E: ElementType,
{
    /// Fills the container with the element.
    fn load(self, el: &crate::renderer::types::Element);
}

/// An [`Attribute`] that will fill a [`NodeRefContainer`] with an HTML element.
#[derive(Debug)]
pub struct NodeRefAttr<E, C> {
    container: C,
    ty: PhantomData<E>,
}

impl<E, C> Clone for NodeRefAttr<E, C>
where
    C: Clone,
{
    fn clone(&self) -> Self {
        Self {
            container: self.container.clone(),
            ty: PhantomData,
        }
    }
}

/// Creates an attribute that will fill a [`NodeRefContainer`] with the element it is applied to.
pub fn node_ref<E, C>(container: C) -> NodeRefAttr<E, C>
where
    E: ElementType,
    C: NodeRefContainer<E>,
{
    NodeRefAttr {
        container,
        ty: PhantomData,
    }
}

impl<E, C> Attribute for NodeRefAttr<E, C>
where
    E: ElementType,
    C: NodeRefContainer<E>,

    crate::renderer::types::Element: PartialEq,
{
    const MIN_LENGTH: usize = 0;
    type AsyncOutput = Self;
    type State = crate::renderer::types::Element;
    type Cloneable = PanicOnCloneAttr<Self>;
    type CloneableOwned = PanicOnCloneAttr<Self>;

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
        self.container.load(el);
        el.to_owned()
    }

    fn build(self, el: &crate::renderer::types::Element) -> Self::State {
        self.container.load(el);
        el.to_owned()
    }

    fn rebuild(self, state: &mut Self::State) {
        self.container.load(state);
    }

    fn into_cloneable(self) -> Self::Cloneable {
        PanicOnCloneAttr::new(
            self,
            "node_ref should not be spread across multiple elements.",
        )
    }

    fn into_cloneable_owned(self) -> Self::Cloneable {
        PanicOnCloneAttr::new(
            self,
            "node_ref should not be spread across multiple elements.",
        )
    }

    fn dry_resolve(&mut self) {}

    async fn resolve(self) -> Self::AsyncOutput {
        self
    }
}

impl<E, C> NextAttribute for NodeRefAttr<E, C>
where
    E: ElementType,
    C: NodeRefContainer<E>,

    crate::renderer::types::Element: PartialEq,
{
    next_attr_output_type!(Self, NewAttr);

    fn add_any_attr<NewAttr: Attribute>(
        self,
        new_attr: NewAttr,
    ) -> Self::Output<NewAttr> {
        next_attr_combine!(self, new_attr)
    }
}

/// Adds the `node_ref` attribute to an element.
pub trait NodeRefAttribute<E, C>
where
    E: ElementType,
    C: NodeRefContainer<E>,

    crate::renderer::types::Element: PartialEq,
{
    /// Binds this HTML element to a [`NodeRefContainer`].
    fn node_ref(
        self,
        container: C,
    ) -> <Self as AddAnyAttr>::Output<NodeRefAttr<E, C>>
    where
        Self: Sized + AddAnyAttr,
        <Self as AddAnyAttr>::Output<NodeRefAttr<E, C>>: Render,
    {
        self.add_any_attr(node_ref(container))
    }
}

impl<E, At, Ch, C> NodeRefAttribute<E, C> for HtmlElement<E, At, Ch>
where
    E: ElementType,
    At: Attribute,
    Ch: Render,
    C: NodeRefContainer<E>,

    crate::renderer::types::Element: PartialEq,
{
}
