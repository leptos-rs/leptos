use crate::{
    html::{element::ElementType, node_ref::NodeRefContainer},
    renderer::{dom::Dom, Renderer},
};
use reactive_graph::{
    signal::RwSignal,
    traits::{DefinedAt, Set, Track, WithUntracked},
};
use send_wrapper::SendWrapper;
use wasm_bindgen::JsCast;

#[derive(Debug)]
pub struct NodeRef<E>(RwSignal<Option<SendWrapper<E::Output>>>)
where
    E: ElementType,
    E::Output: 'static;

impl<E> NodeRef<E>
where
    E: ElementType,
    E::Output: 'static,
{
    #[track_caller]
    pub fn new() -> Self {
        Self(RwSignal::new(None))
    }
}

impl<E> Default for NodeRef<E>
where
    E: ElementType,
    E::Output: 'static,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<E> Clone for NodeRef<E>
where
    E: ElementType,
    E::Output: 'static,
{
    fn clone(&self) -> Self {
        *self
    }
}

impl<E> Copy for NodeRef<E>
where
    E: ElementType,
    E::Output: 'static,
{
}

impl<E> NodeRefContainer<E, Dom> for NodeRef<E>
where
    E: ElementType,
    E::Output: JsCast + 'static,
{
    fn load(self, el: &<Dom as Renderer>::Element) {
        self.0
            .set(Some(SendWrapper::new(el.clone().unchecked_into())));
    }
}

impl<E> DefinedAt for NodeRef<E>
where
    E: ElementType,
    E::Output: JsCast + 'static,
{
    fn defined_at(&self) -> Option<&'static std::panic::Location<'static>> {
        self.0.defined_at()
    }
}

impl<E> WithUntracked for NodeRef<E>
where
    E: ElementType,
    E::Output: JsCast + Clone + 'static,
{
    type Value = Option<E::Output>;

    fn try_with_untracked<U>(
        &self,
        fun: impl FnOnce(&Self::Value) -> U,
    ) -> Option<U> {
        self.0
            .try_with_untracked(|inner| fun(&inner.as_deref().cloned()))
    }
}

impl<E> Track for NodeRef<E>
where
    E: ElementType,
    E::Output: JsCast + 'static,
{
    fn track(&self) {
        self.0.track();
    }
}
