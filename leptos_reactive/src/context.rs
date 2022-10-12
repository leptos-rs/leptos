use std::any::{type_name, Any, TypeId};

use crate::Scope;

/// Provides a context value of type `T` to the current reactive [Scope](crate::Scope)
/// and all of its descendants. This can be consumed using [use_context](crate::use_context).
///
/// This is useful for passing values down to components or functions lower in a
/// hierarchy without needs to “prop drill” by passing them through each layer as
/// arguments to a function or properties of a component.
///
/// ```
/// # use leptos_reactive::*;
/// # create_scope(|cx| {
///
/// // Note: this example doesn’t use Leptos’s DOM model or component structure,
/// // so it ends up being a little silly.
///
/// #[derive(Clone)]
/// struct SharedData {
///   name: (ReadSignal<String>, WriteSignal<String>)
/// }
///
/// let my_context_obj = SharedData { name: create_signal(cx, "Greg".to_string()) };
/// provide_context(cx, my_context_obj);
///
/// // we can access it in this Scope
/// let shared_data = use_context::<SharedData>(cx).unwrap();
/// let (name, set_name) = shared_data.name;
///
/// // we can access it somewhere in a lower scope
/// cx.child_scope(|cx| {
///   let shared_data_lower_in_tree = use_context::<SharedData>(cx).unwrap();
///   let (name, set_name) = shared_data.name;
///   set_name("Bob".to_string());
/// });
///
/// // the change made in a lower scope updated the signal in the parent scope
/// assert_eq!(name(), "Bob");
///
/// # }).dispose();
/// ```
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

/// Extracts a context value of type `T` from the reactive system by traversing
/// it upwards, beginning from the current [Scope](crate::Scope) and iterating
/// through its parents, if any. The context value should have been provided elsewhere
/// using [provide_context](crate::provide_context).
///
/// This is useful for passing values down to components or functions lower in a
/// hierarchy without needs to “prop drill” by passing them through each layer as
/// arguments to a function or properties of a component.
///
/// ```
/// # use leptos_reactive::*;
/// # create_scope(|cx| {
///
/// // Note: this example doesn’t use Leptos’s DOM model or component structure,
/// // so it ends up being a little silly.
///
/// #[derive(Clone)]
/// struct SharedData {
///   name: (ReadSignal<String>, WriteSignal<String>)
/// }
///
/// let my_context_obj = SharedData { name: create_signal(cx, "Greg".to_string()) };
/// provide_context(cx, my_context_obj);
///
/// // we can access it in this Scope
/// let shared_data = use_context::<SharedData>(cx).unwrap();
/// let (name, set_name) = shared_data.name;
///
/// // we can access it somewhere in a lower scope
/// cx.child_scope(|cx| {
///   let shared_data_lower_in_tree = use_context::<SharedData>(cx).unwrap();
///   let (name, set_name) = shared_data.name;
///   set_name("Bob".to_string());
/// });
///
/// // the change made in a lower scope updated the signal in the parent scope
/// assert_eq!(name(), "Bob");
///
/// # }).dispose();
/// ```
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
