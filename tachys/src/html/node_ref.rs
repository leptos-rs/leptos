use super::{
    attribute::{global::AddAttribute, Attribute},
    element::ElementType,
};
use crate::{html::element::HtmlElement, prelude::Render, renderer::Renderer};
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
    Rndr::Element: Clone + PartialEq,
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

pub trait NodeRefAttribute<E, C, Rndr>
where
    E: ElementType,
    C: NodeRefContainer<E, Rndr>,
    Rndr: Renderer,
{
    fn node_ref(
        self,
        container: C,
    ) -> <Self as AddAttribute<NodeRefAttr<E, C, Rndr>, Rndr>>::Output
    where
        Self: Sized + AddAttribute<NodeRefAttr<E, C, Rndr>, Rndr>,
        <Self as AddAttribute<NodeRefAttr<E, C, Rndr>, Rndr>>::Output:
            Render<Rndr>,
    {
        self.add_attr(node_ref(container))
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
{
}
