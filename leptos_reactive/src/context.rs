use std::any::{Any, TypeId};

use crate::Scope;

pub fn provide_context<T>(cx: Scope, value: T)
where
    T: Clone + 'static,
{
    let id = value.type_id();
    cx.runtime.scope(cx.id, |scope_state| {
        scope_state
            .contexts
            .borrow_mut()
            .insert(id, Box::new(value));
    })
}

pub fn use_context<T>(cx: Scope) -> Option<T>
where
    T: Clone + 'static,
{
    let id = TypeId::of::<T>();
    cx.runtime.scope(cx.id, |scope_state| {
        let contexts = scope_state.contexts.borrow();
        let local_value = contexts.get(&id).and_then(|val| val.downcast_ref::<T>());
        match local_value {
            Some(val) => Some(val.clone()),
            None => scope_state
                .parent
                .and_then(|parent| use_context::<T>(parent)),
        }
    })
}
