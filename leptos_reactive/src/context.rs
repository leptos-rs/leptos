use std::any::{Any, TypeId};

use crate::Scope;

impl Scope {
    pub fn provide_context<T>(self, value: T)
    where
        T: Clone + 'static,
    {
        let id = value.type_id();
        self.runtime.scope(self.id, |scope_state| {
            scope_state
                .contexts
                .borrow_mut()
                .insert(id, Box::new(value));
        })
    }

    pub fn use_context<T>(self) -> Option<T>
    where
        T: Clone + 'static,
    {
        let id = TypeId::of::<T>();
        self.runtime.scope(self.id, |scope_state| {
            let contexts = scope_state.contexts.borrow();
            let local_value = contexts.get(&id).and_then(|val| val.downcast_ref::<T>());
            match local_value {
                Some(val) => Some(val.clone()),
                None => scope_state
                    .parent
                    .and_then(|parent| parent.use_context::<T>()),
            }
        })
    }
}
