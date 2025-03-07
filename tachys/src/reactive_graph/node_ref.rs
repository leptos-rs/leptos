use crate::html::{element::ElementType, node_ref::NodeRefContainer};
use reactive_graph::{
    effect::Effect,
    graph::untrack,
    signal::{
        guards::{Derefable, ReadGuard},
        RwSignal,
    },
    traits::{
        DefinedAt, Get, Notify, ReadUntracked, Set, Track, UntrackableGuard,
        Write,
    },
};
use send_wrapper::SendWrapper;
use std::{cell::Cell, ops::DerefMut};
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

    /// Runs the provided closure when the `NodeRef` has been connected
    /// with its element.
    #[inline(always)]
    pub fn on_load<F>(self, f: F)
    where
        E: 'static,
        F: FnOnce(E::Output) + 'static,
        E: ElementType,
        E::Output: JsCast + Clone + 'static,
    {
        let f = Cell::new(Some(f));

        Effect::new(move |_| {
            if let Some(node_ref) = self.get() {
                let f = f.take().unwrap();
                untrack(move || {
                    f(node_ref);
                });
            }
        });
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

impl<E> Notify for NodeRef<E>
where
    E: ElementType,
    E::Output: JsCast + Clone + 'static,
{
    fn notify(&self) {
        self.0.notify();
    }
}

impl<E> Write for NodeRef<E>
where
    E: ElementType,
    E::Output: JsCast + Clone + 'static,
{
    type Value = Option<SendWrapper<E::Output>>;

    fn try_write(&self) -> Option<impl UntrackableGuard<Target = Self::Value>> {
        self.0.try_write()
    }

    fn try_write_untracked(
        &self,
    ) -> Option<impl DerefMut<Target = Self::Value>> {
        self.0.try_write_untracked()
    }
}

impl<E> ReadUntracked for NodeRef<E>
where
    E: ElementType,
    E::Output: JsCast + Clone + 'static,
{
    type Value = ReadGuard<Option<E::Output>, Derefable<Option<E::Output>>>;

    fn try_read_untracked(&self) -> Option<Self::Value> {
        Some(ReadGuard::new(Derefable(
            self.0.try_read_untracked()?.as_deref().cloned(),
        )))
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
