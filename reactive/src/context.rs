use std::any::{Any, TypeId};

use crate::{BoundedScope, ScopeInner};

impl<'a, 'b> BoundedScope<'a, 'b> {
    pub fn provide_context<T: 'static>(self, value: T) {
        let id = value.type_id();
        let value = self.inner.arena.alloc(value);
        self.inner.context.borrow_mut().insert(id, &*value);
    }

    pub fn use_context<T: 'static>(self) -> Option<&'a T> {
        self.inner.use_context()
    }
}

impl<'a> ScopeInner<'a> {
    pub fn use_context<T: 'static>(&'a self) -> Option<&T> {
        let id = TypeId::of::<T>();
        let local_value = self
            .context
            .borrow()
            .get(&id)
            .and_then(|val| val.downcast_ref::<T>());
        match local_value {
            Some(val) => Some(val),
            None => self.parent.and_then(|parent| parent.use_context::<T>()),
        }
    }
}
