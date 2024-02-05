use crate::owner::Owner;
use or_poisoned::OrPoisoned;
use std::any::{Any, TypeId};

impl Owner {
    fn provide_context<T: Send + Sync + 'static>(&self, value: T) {
        self.inner
            .write()
            .or_poisoned()
            .contexts
            .insert(value.type_id(), Box::new(value));
    }

    fn use_context<T: Clone + 'static>(&self) -> Option<T> {
        let ty = TypeId::of::<T>();
        let inner = self.inner.read().or_poisoned();
        let mut parent = inner.parent.as_ref().and_then(|p| p.upgrade());
        let contexts = &self.inner.read().or_poisoned().contexts;
        if let Some(context) = contexts.get(&ty) {
            context.downcast_ref::<T>().cloned()
        } else {
            while let Some(ref this_parent) = parent.clone() {
                let this_parent = this_parent.read().or_poisoned();
                let contexts = &this_parent.contexts;
                let value = contexts.get(&ty);
                let downcast = value
                    .and_then(|context| context.downcast_ref::<T>().cloned());
                if let Some(value) = downcast {
                    return Some(value);
                } else {
                    parent =
                        this_parent.parent.as_ref().and_then(|p| p.upgrade());
                }
            }
            None
        }
    }
}

pub fn provide_context<T: Send + Sync + 'static>(value: T) {
    if let Some(owner) = Owner::current() {
        owner.provide_context(value);
    }
}

pub fn use_context<T: Clone + 'static>() -> Option<T> {
    Owner::current().and_then(|owner| owner.use_context())
}
