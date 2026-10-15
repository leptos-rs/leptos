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
        self.with_context(Clone::clone)
    }

    // The three lookups below release `self`'s lock before walking up to its
    // ancestors, and drop each ancestor's lock before its `Arc`. The walk
    // upgrades `Weak` parents; if one of those upgrades ends up holding an
    // ancestor's last strong reference (the ancestor was dropped elsewhere
    // meanwhile), dropping it runs that ancestor's cleanup, which write-locks
    // its children, `self` among them. Held across that drop, `self`'s lock
    // deadlocked the thread (#4946).

    fn take_context<T: 'static>(&self) -> Option<T> {
        let ty = TypeId::of::<T>();
        let mut parent = {
            let mut inner = self.inner.write().or_poisoned();
            if let Some(context) = inner.contexts.remove(&ty) {
                return context.downcast::<T>().ok().map(|n| *n);
            }
            inner.parent.as_ref().and_then(|p| p.upgrade())
        };
        while let Some(this_parent) = parent.take() {
            parent = {
                let mut this_parent = this_parent.write().or_poisoned();
                let value = this_parent.contexts.remove(&ty);
                if let Some(value) =
                    value.and_then(|context| context.downcast::<T>().ok())
                {
                    return Some(*value);
                }
                this_parent.parent.as_ref().and_then(|p| p.upgrade())
            };
        }
        None
    }

    fn with_context<T: 'static, R>(
        &self,
        cb: impl FnOnce(&T) -> R,
    ) -> Option<R> {
        let ty = TypeId::of::<T>();
        let mut parent = {
            let inner = self.inner.read().or_poisoned();
            if let Some(value) = inner
                .contexts
                .get(&ty)
                .and_then(|context| context.downcast_ref::<T>())
            {
                return Some(cb(value));
            }
            inner.parent.as_ref().and_then(|p| p.upgrade())
        };
        while let Some(this_parent) = parent.take() {
            parent = {
                let this_parent = this_parent.read().or_poisoned();
                if let Some(value) = this_parent
                    .contexts
                    .get(&ty)
                    .and_then(|context| context.downcast_ref::<T>())
                {
                    return Some(cb(value));
                }
                this_parent.parent.as_ref().and_then(|p| p.upgrade())
            };
        }
        None
    }

    fn update_context<T: 'static, R>(
        &self,
        cb: impl FnOnce(&mut T) -> R,
    ) -> Option<R> {
        let ty = TypeId::of::<T>();
        let mut parent = {
            let mut inner = self.inner.write().or_poisoned();
            if let Some(value) = inner
                .contexts
                .get_mut(&ty)
                .and_then(|context| context.downcast_mut::<T>())
            {
                return Some(cb(value));
            }
            inner.parent.as_ref().and_then(|p| p.upgrade())
        };
        while let Some(this_parent) = parent.take() {
            parent = {
                let mut this_parent = this_parent.write().or_poisoned();
                if let Some(value) = this_parent
                    .contexts
                    .get_mut(&ty)
                    .and_then(|context| context.downcast_mut::<T>())
                {
                    return Some(cb(value));
                }
                this_parent.parent.as_ref().and_then(|p| p.upgrade())
            };
        }
        None
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

/// Extracts a context value of type `T` from the reactive system.
///
/// This traverses the reactive ownership graph, beginning from the current reactive
/// [`Owner`] and iterating through its parents, if any. When the value is found, it is cloned.
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
///         let value = use_context::<String>()
///             .expect("could not find String in context");
///         assert_eq!(value, "foo");
///         let value2 = use_context::<String>()
///             .expect("could not find String in context");
///         assert_eq!(value2, "foo");
///     });
/// });
/// # });
/// ```
pub fn use_context<T: Clone + 'static>() -> Option<T> {
    Owner::current().and_then(|owner| owner.use_context())
}

/// Extracts a context value of type `T` from the reactive system, and
/// panics if it can't be found.
///
/// This traverses the reactive ownership graph, beginning from the current reactive
/// [`Owner`] and iterating through its parents, if any. When the value is found, it is cloned.
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
///         let value = use_context::<String>()
///             .expect("could not find String in context");
///         assert_eq!(value, "foo");
///         let value2 = use_context::<String>()
///             .expect("could not find String in context");
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

/// Extracts a context value of type `T` from the reactive system, and takes ownership,
/// removing it from the context system.
///
/// This traverses the reactive ownership graph, beginning from the current reactive
/// [`Owner`] and iterating through its parents, if any. When the value is found, it is removed,
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

/// Access a reference to a context value of type `T` in the reactive system.
///
/// This traverses the reactive ownership graph, beginning from the current reactive
/// [`Owner`] and iterating through its parents, if any. When the value is found,
/// the function that you pass is applied to an immutable reference to it.
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
///         let value = with_context::<String, _>(|val| val.to_string())
///             .expect("could not find String in context");
///         assert_eq!(value, "foo");
///     });
/// });
/// # });
/// ```
pub fn with_context<T: 'static, R>(cb: impl FnOnce(&T) -> R) -> Option<R> {
    Owner::current().and_then(|owner| owner.with_context(cb))
}

/// Update a context value of type `T` in the reactive system.
///
/// This traverses the reactive ownership graph, beginning from the current reactive
/// [`Owner`] and iterating through its parents, if any. When the value is found,
/// the function that you pass is applied to a mutable reference to it.
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
///         let value = update_context::<String, _>(|val| {
///             std::mem::replace(val, "bar".to_string())
///         })
///         .expect("could not find String in context");
///         assert_eq!(value, "foo");
///         assert_eq!(expect_context::<String>(), "bar");
///     });
/// });
/// # });
/// ```
pub fn update_context<T: 'static, R>(
    cb: impl FnOnce(&mut T) -> R,
) -> Option<R> {
    Owner::current().and_then(|owner| owner.update_context(cb))
}

#[cfg(test)]
mod tests {
    use crate::owner::Owner;
    use std::{
        sync::{Arc, Mutex, mpsc},
        thread,
        time::Duration,
    };

    fn does_not_deadlock(f: impl FnOnce() + Send + 'static) -> bool {
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            f();
            _ = tx.send(());
        });
        rx.recv_timeout(Duration::from_secs(5)).is_ok()
    }

    struct Marker;

    /// A child owner whose parent provides `Marker`, and whose parent's only
    /// strong handle is in the returned slot (the child holds a `Weak`).
    /// Emptying the slot during a lookup from the child leaves the lookup's
    /// own upgraded `Arc` as the parent's last strong reference, the way a
    /// parent being dropped on another thread would.
    fn child_of_droppable_parent() -> (Owner, Arc<Mutex<Option<Owner>>>) {
        let parent = Owner::new();
        parent.provide_context(Marker);
        let child = parent.child();
        (child, Arc::new(Mutex::new(Some(parent))))
    }

    #[test]
    fn with_context_does_not_deadlock_when_the_lookup_drops_the_last_parent() {
        assert!(does_not_deadlock(|| {
            let (child, parent) = child_of_droppable_parent();
            let found = child.with_context::<Marker, _>(|_| {
                parent.lock().unwrap().take();
            });
            assert!(found.is_some());
        }));
    }

    #[test]
    fn update_context_does_not_deadlock_when_the_lookup_drops_the_last_parent()
    {
        assert!(does_not_deadlock(|| {
            let (child, parent) = child_of_droppable_parent();
            let found = child.update_context::<Marker, _>(|_| {
                parent.lock().unwrap().take();
            });
            assert!(found.is_some());
        }));
    }
}
