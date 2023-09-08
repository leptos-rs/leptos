use elsa::FrozenVec;
use std::{
    any::Any,
    cell::{Cell, Ref, RefCell, RefMut},
    fmt::Debug,
    marker::PhantomData,
    rc::Rc,
};

#[derive(Clone)]
pub(crate) struct Arena {
    #[allow(clippy::type_complexity)]
    values: &'static FrozenVec<Box<Slot>>,
    open: Rc<RefCell<Vec<u32>>>,
}

impl Arena {
    pub fn new() -> Self {
        Self {
            values: Box::leak(Box::new(FrozenVec::new())),
            open: Default::default(),
        }
    }

    fn push<T: 'static>(&self, value: T) -> NodeId {
        self.push_erased(Box::new(value))
    }

    pub fn get<T: 'static>(&self, id: &NodeId) -> Option<Ref<'_, T>> {
        let node = self.get_any(id)?;
        Ref::filter_map(node, |node| {
            let node = node.as_ref()?;
            match node.downcast_ref::<T>() {
                Some(t) => Some(t),
                None => None,
            }
        })
        .ok()
    }

    pub fn get_mut<T: 'static>(&self, id: &NodeId) -> Option<RefMut<'_, T>> {
        let node = self.get_any_mut(id)?;
        RefMut::filter_map(node, |node| {
            let node = node.as_mut()?;
            match node.downcast_mut::<T>() {
                Some(t) => Some(t),
                None => None,
            }
        })
        .ok()
    }

    pub fn remove(&self, id: &NodeId) -> Option<Box<dyn Any>> {
        self.recycle(id)
    }

    fn get_any(&self, id: &NodeId) -> Option<Ref<'_, Option<Box<dyn Any>>>> {
        let node = self.values.get(id.idx as usize)?;
        if id.generation == node.generation.get() {
            Some(node.value.borrow())
        } else {
            None
        }
    }

    pub fn get_any_mut(
        &self,
        id: &NodeId,
    ) -> Option<RefMut<'_, Option<Box<dyn Any>>>> {
        let node = self.values.get(id.idx as usize)?;
        if id.generation == node.generation.get() {
            Some(node.value.borrow_mut())
        } else {
            None
        }
    }

    fn recycle(&self, id: &NodeId) -> Option<Box<dyn Any>> {
        let node = self.values.get(id.idx as usize)?;
        node.value.borrow_mut().take()
    }

    fn push_erased(&self, value: Box<dyn Any>) -> NodeId {
        if let Some(next_node) = self.open.borrow_mut().pop() {
            let slot = self.values.get(next_node as usize).unwrap();
            let generation = slot.generation.get() + 1;
            slot.generation.set(generation);
            *slot.value.borrow_mut() = Some(value);
            NodeId {
                idx: next_node,
                generation,
            }
        } else {
            self.values.push(Box::new(Slot {
                generation: Cell::new(0),
                value: RefCell::new(Some(value)),
            }));
            let idx = (self.values.len() - 1) as u32;
            NodeId { idx, generation: 0 }
        }
    }
}

impl Debug for Arena {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Arena")
            .field("values", &self.values.len())
            .field("open", &self.open)
            .finish()
    }
}

#[derive(Copy, Clone, Default, Debug, PartialEq, Eq, Hash)]
/// TODO
pub struct NodeId {
    idx: u32,
    generation: u32,
}

#[derive(Debug)]
pub(crate) struct Slot {
    generation: Cell<u32>,
    value: RefCell<Option<Box<dyn Any>>>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct Value<T> {
    id: NodeId,
    ty: PhantomData<T>,
}

#[derive(Debug)]
pub(crate) struct ArenaOwner {
    pub(crate) arena: &'static Arena,
    pub(crate) owned: RefCell<Vec<NodeId>>,
}

impl ArenaOwner {
    pub fn insert<T: 'static>(&self, value: T) -> NodeId {
        let id = self.arena.push(value);
        self.register(id)
    }

    pub fn insert_boxed(&self, value: Box<dyn Any>) -> NodeId {
        let id = self.arena.push_erased(value);
        self.register(id)
    }

    fn register(&self, id: NodeId) -> NodeId {
        self.owned.borrow_mut().push(id);
        id
    }
}

impl Drop for ArenaOwner {
    fn drop(&mut self) {
        for location in self.owned.borrow().iter() {
            drop(self.arena.recycle(location));
        }
    }
}
