use super::{
    attribute::{Attribute, NextAttribute},
    element::ElementType,
};
use crate::{
    html::element::HtmlElement, prelude::Render, renderer::Renderer,
    view::add_attr::AddAnyAttr,
};
use std::marker::PhantomData;

pub trait NodeRefContainer<E, Rndr>
where
    E: ElementType,
    Rndr: Renderer,
{
    fn load(self, el: &Rndr::Element);
}

pub struct NodeRefAttr<E, C, Rndr>
where
    E: ElementType,
    C: NodeRefContainer<E, Rndr>,
    Rndr: Renderer,
{
    container: C,
    ty: PhantomData<E>,
    rndr: PhantomData<Rndr>,
}

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
    type State = ();

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
    }

    fn build(self, el: &<Rndr as Renderer>::Element) -> Self::State {
        self.container.load(el);
    }

    fn rebuild(self, _state: &mut Self::State) {}
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

pub trait NodeRefAttribute<E, C, Rndr>
where
    E: ElementType,
    C: NodeRefContainer<E, Rndr>,
    Rndr: Renderer,
    Rndr::Element: PartialEq,
{
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
