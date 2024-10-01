use crate::html::{element::ElementType, node_ref::NodeRefContainer};
use reactive_graph::{
    signal::RwSignal,
    traits::{DefinedAt, Set, Track, WithUntracked},
};
use send_wrapper::SendWrapper;
use wasm_bindgen::JsCast;

/// A reactive reference to a DOM node that can be used with the `node_ref` attribute.
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
    /// Creates a new node reference.
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

impl<E> NodeRefContainer<E> for NodeRef<E>
where
    E: ElementType,
    E::Output: JsCast + 'static,
{
    fn load(self, el: &crate::renderer::types::Element) {
        // safe to construct SendWrapper here, because it will only run in the browser
        // so it will always be accessed or dropped from the main thread
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

/// Create a [NodeRef].
#[inline(always)]
#[track_caller]
#[deprecated = "This function is being removed to conform to Rust idioms. \
                Please use `NodeRef::new()` instead."]
pub fn create_node_ref<E>() -> NodeRef<E>
where
    E: ElementType,
    E::Output: 'static,
{
    NodeRef::new()
}
