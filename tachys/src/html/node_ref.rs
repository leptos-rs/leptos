use super::{
    attribute::{Attribute, NextAttribute},
    element::ElementType,
};
use crate::{
    html::element::HtmlElement, prelude::Render, renderer::Renderer,
    view::add_attr::AddAnyAttr,
};
use std::marker::PhantomData;

/// Describes a container that can be used to hold a reference to an HTML element.
pub trait NodeRefContainer<E, Rndr>: Send + Clone
where
    E: ElementType,
    Rndr: Renderer,
{
    /// Fills the container with the element.
    fn load(self, el: &Rndr::Element);
}

/// An [`Attribute`] that will fill a [`NodeRefContainer`] with an HTML element.
#[derive(Debug)]
pub struct NodeRefAttr<E, C, Rndr> {
    container: C,
    ty: PhantomData<E>,
    rndr: PhantomData<Rndr>,
}

impl<E, C, Rndr> Clone for NodeRefAttr<E, C, Rndr>
where
    C: Clone,
{
    fn clone(&self) -> Self {
        Self {
            container: self.container.clone(),
            ty: PhantomData,
            rndr: PhantomData,
        }
    }
}

/// Creates an attribute that will fill a [`NodeRefContainer`] with the element it is applied to.
pub fn node_ref<E, C, Rndr>(container: C) -> NodeRefAttr<E, C, Rndr>
where
    E: ElementType,
    C: NodeRefContainer<E, Rndr>,
    Rndr: Renderer,
{
    NodeRefAttr {
        container,
        ty: PhantomData,
        rndr: PhantomData,
    }
}

impl<E, C, Rndr> Attribute<Rndr> for NodeRefAttr<E, C, Rndr>
where
    E: ElementType,
    C: NodeRefContainer<E, Rndr>,
    Rndr: Renderer,
    Rndr::Element: PartialEq,
{
    const MIN_LENGTH: usize = 0;
    type AsyncOutput = Self;
    type State = Rndr::Element;
    type Cloneable = ();
    type CloneableOwned = ();

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
        el: &<Rndr as Renderer>::Element,
    ) -> Self::State {
        self.container.load(el);
        el.to_owned()
    }

    fn build(self, el: &<Rndr as Renderer>::Element) -> Self::State {
        self.container.load(el);
        el.to_owned()
    }

    fn rebuild(self, state: &mut Self::State) {
        self.container.load(state);
    }

    fn into_cloneable(self) -> Self::Cloneable {
        panic!("node_ref should not be spread across multiple elements.");
    }

    fn into_cloneable_owned(self) -> Self::Cloneable {
        panic!("node_ref should not be spread across multiple elements.");
    }

    fn dry_resolve(&mut self) {}

    async fn resolve(self) -> Self::AsyncOutput {
        self
    }
}

impl<E, C, Rndr> NextAttribute<Rndr> for NodeRefAttr<E, C, Rndr>
where
    E: ElementType,
    C: NodeRefContainer<E, Rndr>,
    Rndr: Renderer,
    Rndr::Element: PartialEq,
{
    type Output<NewAttr: Attribute<Rndr>> = (Self, NewAttr);

    fn add_any_attr<NewAttr: Attribute<Rndr>>(
        self,
        new_attr: NewAttr,
    ) -> Self::Output<NewAttr> {
        (self, new_attr)
    }
}

/// Adds the `node_ref` attribute to an element.
pub trait NodeRefAttribute<E, C, Rndr>
where
    E: ElementType,
    C: NodeRefContainer<E, Rndr>,
    Rndr: Renderer,
    Rndr::Element: PartialEq,
{
    /// Binds this HTML element to a [`NodeRefContainer`].
    fn node_ref(
        self,
        container: C,
    ) -> <Self as AddAnyAttr<Rndr>>::Output<NodeRefAttr<E, C, Rndr>>
    where
        Self: Sized + AddAnyAttr<Rndr>,
        <Self as AddAnyAttr<Rndr>>::Output<NodeRefAttr<E, C, Rndr>>:
            Render<Rndr>,
    {
        self.add_any_attr(node_ref(container))
    }
}

impl<E, At, Ch, C, Rndr> NodeRefAttribute<E, C, Rndr>
    for HtmlElement<E, At, Ch, Rndr>
where
    E: ElementType,
    At: Attribute<Rndr>,
    Ch: Render<Rndr>,
    C: NodeRefContainer<E, Rndr>,
    Rndr: Renderer,
    Rndr::Element: PartialEq,
{
}
