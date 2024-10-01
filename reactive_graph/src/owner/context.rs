use crate::owner::Owner;
use or_poisoned::OrPoisoned;
use std::{
    any::{Any, TypeId},
    collections::VecDeque,
};

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

    fn take_context<T: 'static>(&self) -> Option<T> {
        let ty = TypeId::of::<T>();
        let inner = self.inner.read().or_poisoned();
        let mut parent = inner.parent.as_ref().and_then(|p| p.upgrade());
        let contexts = &mut self.inner.write().or_poisoned().contexts;
        if let Some(context) = contexts.remove(&ty) {
            context.downcast::<T>().ok().map(|n| *n)
        } else {
            while let Some(ref this_parent) = parent.clone() {
                let mut this_parent = this_parent.write().or_poisoned();
                let contexts = &mut this_parent.contexts;
                let value = contexts.remove(&ty);
                let downcast =
                    value.and_then(|context| context.downcast::<T>().ok());
                if let Some(value) = downcast {
                    return Some(*value);
                } else {
                    parent =
                        this_parent.parent.as_ref().and_then(|p| p.upgrade());
                }
            }
            None
        }
    }

    /// Searches for items stored in context in either direction, either among parents or among
    /// descendants.
    pub fn use_context_bidirectional<T: Clone + 'static>(&self) -> Option<T> {
        self.use_context()
            .unwrap_or_else(|| self.find_context_in_children())
    }

    fn find_context_in_children<T: Clone + 'static>(&self) -> Option<T> {
        let ty = TypeId::of::<T>();
        let inner = self.inner.read().or_poisoned();
        let mut to_search = VecDeque::new();
        to_search.extend(inner.children.clone());
        drop(inner);

        while let Some(next) = to_search.pop_front() {
            if let Some(child) = next.upgrade() {
                let child = child.read().or_poisoned();
                let contexts = &child.contexts;
                if let Some(context) = contexts.get(&ty) {
                    return context.downcast_ref::<T>().cloned();
                }

                to_search.extend(child.children.clone());
            }
        }

        None
    }
}

/// Provides a context value of type `T` to the current reactive [`Owner`]
/// and all of its descendants. This can be accessed using [`use_context`].
///
/// This is useful for passing values down to components or functions lower in a
/// hierarchy without needs to “prop drill” by passing them through each layer as
/// arguments to a function or properties of a component.
///
/// Context works similarly to variable scope: a context that is provided higher in
/// the reactive graph can be used lower down, but a context that is provided lower
/// down cannot be used higher up.
///
/// ```rust
/// # use reactive_graph::prelude::*;
/// # use reactive_graph::owner::*;
/// # let owner = Owner::new(); owner.set();
/// # use reactive_graph::effect::Effect;
/// # futures::executor::block_on(async move {
/// # any_spawner::Executor::init_futures_executor();
/// Effect::new(move |_| {
///     println!("Provider");
///     provide_context(42i32); // provide an i32
///
///     Effect::new(move |_| {
///         println!("intermediate node");
///
///         Effect::new(move |_| {
///             let value = use_context::<i32>()
///                 .expect("could not find i32 in context");
///             assert_eq!(value, 42);
///         });
///     });
/// });
/// # });
/// ```
///
/// ## Context Shadowing
///
/// Only a single value of any type can be provided via context. If you need to provide multiple
/// values of the same type, wrap each one in a "newtype" struct wrapper so that each one is a
/// distinct type.
///
/// Providing a second value of the same type "lower" in the ownership tree will shadow the value,
/// just as a second `let` declaration with the same variable name will shadow that variable.
///
/// ```rust
/// # use reactive_graph::prelude::*;
/// # use reactive_graph::owner::*;
/// # let owner = Owner::new(); owner.set();
/// # use reactive_graph::effect::Effect;
/// # futures::executor::block_on(async move {
/// # any_spawner::Executor::init_futures_executor();
/// Effect::new(move |_| {
///     println!("Provider");
///     provide_context("foo"); // provide a &'static str
///
///     Effect::new(move |_| {
///         // before we provide another value of the same type, we can access the old one
///         assert_eq!(use_context::<&'static str>(), Some("foo"));
///         // but providing another value of the same type shadows it
///         provide_context("bar");
///
///         Effect::new(move |_| {
///             assert_eq!(use_context::<&'static str>(), Some("bar"));
///         });
///     });
/// });
/// # });
/// ```
pub fn provide_context<T: Send + Sync + 'static>(value: T) {
    if let Some(owner) = Owner::current() {
        owner.provide_context(value);
    }
}

/// Extracts a context value of type `T` from the reactive system by traversing
/// it upwards, beginning from the current reactive [`Owner`] and iterating
/// through its parents, if any. When the value is found, it is cloned.
///
/// The context value should have been provided elsewhere using
/// [`provide_context`](provide_context).
///
/// This is useful for passing values down to components or functions lower in a
/// hierarchy without needs to “prop drill” by passing them through each layer as
/// arguments to a function or properties of a component.
///
/// Context works similarly to variable scope: a context that is provided higher in
/// the reactive graph can be used lower down, but a context that is provided lower
/// in the tree cannot be used higher up.
///
/// While the term “consume” is sometimes used, note that [`use_context`] clones the value, rather
/// than removing it; it is still accessible to other users.
///
/// ```rust
/// # use reactive_graph::prelude::*;
/// # use reactive_graph::owner::*;
/// # let owner = Owner::new(); owner.set();
/// # use reactive_graph::effect::Effect;
/// # futures::executor::block_on(async move {
/// # any_spawner::Executor::init_futures_executor();
/// Effect::new(move |_| {
///     provide_context(String::from("foo"));
///
///     Effect::new(move |_| {
///         // each use_context clones the value
///         let value =
///             use_context::<String>().expect("could not find i32 in context");
///         assert_eq!(value, "foo");
///         let value2 =
///             use_context::<String>().expect("could not find i32 in context");
///         assert_eq!(value2, "foo");
///     });
/// });
/// # });
/// ```
pub fn use_context<T: Clone + 'static>() -> Option<T> {
    Owner::current().and_then(|owner| owner.use_context())
}

/// Extracts a context value of type `T` from the reactive system by traversing
/// it upwards, beginning from the current reactive [`Owner`] and iterating
/// through its parents, if any. When the value is found, it is cloned.
///
/// Panics if no value is found.
///
/// The context value should have been provided elsewhere using
/// [`provide_context`](provide_context).
///
/// This is useful for passing values down to components or functions lower in a
/// hierarchy without needs to “prop drill” by passing them through each layer as
/// arguments to a function or properties of a component.
///
/// Context works similarly to variable scope: a context that is provided higher in
/// the reactive graph can be used lower down, but a context that is provided lower
/// in the tree cannot be used higher up.
///
/// While the term “consume” is sometimes used, note that [`use_context`] clones the value, rather
/// than removing it; it is still accessible to other users.
///
/// ```rust
/// # use reactive_graph::prelude::*;
/// # use reactive_graph::owner::*;
/// # let owner = Owner::new(); owner.set();
/// # use reactive_graph::effect::Effect;
/// # futures::executor::block_on(async move {
/// # any_spawner::Executor::init_futures_executor();
/// Effect::new(move |_| {
///     provide_context(String::from("foo"));
///
///     Effect::new(move |_| {
///         // each use_context clones the value
///         let value =
///             use_context::<String>().expect("could not find i32 in context");
///         assert_eq!(value, "foo");
///         let value2 =
///             use_context::<String>().expect("could not find i32 in context");
///         assert_eq!(value2, "foo");
///     });
/// });
/// # });
/// ```
/// ## Panics
/// Panics if a context of this type is not found in the current reactive
/// owner or its ancestors.
#[track_caller]
pub fn expect_context<T: Clone + 'static>() -> T {
    let location = std::panic::Location::caller();

    use_context().unwrap_or_else(|| {
        panic!(
            "{:?} expected context of type {:?} to be present",
            location,
            std::any::type_name::<T>()
        )
    })
}

/// Extracts a context value of type `T` from the reactive system by traversing
/// it upwards, beginning from the current reactive [`Owner`] and iterating
/// through its parents, if any. When the value is found, it is removed from the context,
/// and is not available to any other [`use_context`] or [`take_context`] calls.
///
/// If the value is `Clone`, use [`use_context`] instead.
///
/// The context value should have been provided elsewhere using
/// [`provide_context`](provide_context).
///
/// This is useful for passing values down to components or functions lower in a
/// hierarchy without needs to “prop drill” by passing them through each layer as
/// arguments to a function or properties of a component.
///
/// Context works similarly to variable scope: a context that is provided higher in
/// the reactive graph can be used lower down, but a context that is provided lower
/// in the tree cannot be used higher up.
/// ```rust
/// # use reactive_graph::prelude::*;
/// # use reactive_graph::owner::*;
/// # let owner = Owner::new(); owner.set();
/// # use reactive_graph::effect::Effect;
/// # futures::executor::block_on(async move {
/// # any_spawner::Executor::init_futures_executor();
///
/// #[derive(Debug, PartialEq)]
/// struct NotClone(String);
///
/// Effect::new(move |_| {
///     provide_context(NotClone(String::from("foo")));
///
///     Effect::new(move |_| {
///         // take_context removes the value from context without needing to clone
///         let value = take_context::<NotClone>();
///         assert_eq!(value, Some(NotClone(String::from("foo"))));
///         let value2 = take_context::<NotClone>();
///         assert_eq!(value2, None);
///     });
/// });
/// # });
/// ```
pub fn take_context<T: 'static>() -> Option<T> {
    Owner::current().and_then(|owner| owner.take_context())
}
