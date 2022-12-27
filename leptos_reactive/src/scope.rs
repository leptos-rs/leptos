use crate::{
    runtime::{with_runtime, RuntimeId},
    EffectId, PinnedFuture, ResourceId, SignalId, SuspenseContext,
};
use futures::stream::FuturesUnordered;
use std::{collections::HashMap, fmt};

#[doc(hidden)]
#[must_use = "Scope will leak memory if the disposer function is never called"]
/// Creates a new reactive system and root reactive scope and runs the function within it.
///
/// This should usually only be used once, at the root of an application, because its reactive
/// values will not have access to values created under another `create_scope`.
///
/// You usually don't need to call this manually.
pub fn create_scope(runtime: RuntimeId, f: impl FnOnce(Scope) + 'static) -> ScopeDisposer {
    runtime.run_scope_undisposed(f, None).2
}

#[doc(hidden)]
#[must_use = "Scope will leak memory if the disposer function is never called"]
/// Creates a new reactive system and root reactive scope, and returns them.
///
/// This should usually only be used once, at the root of an application, because its reactive
/// values will not have access to values created under another `create_scope`.
///
/// You usually don't need to call this manually.
pub fn raw_scope_and_disposer(runtime: RuntimeId) -> (Scope, ScopeDisposer) {
    runtime.raw_scope_and_disposer()
}

#[doc(hidden)]
/// Creates a temporary scope, runs the given function, disposes of the scope,
/// and returns the value returned from the function. This is very useful for short-lived
/// applications like SSR, where actual reactivity is not required beyond the end
/// of the synchronous operation.
///
/// You usually don't need to call this manually.
pub fn run_scope<T>(runtime: RuntimeId, f: impl FnOnce(Scope) -> T + 'static) -> T {
    runtime.run_scope(f, None)
}

#[doc(hidden)]
#[must_use = "Scope will leak memory if the disposer function is never called"]
/// Creates a temporary scope and run the given function without disposing of the scope.
/// If you do not dispose of the scope on your own, memory will leak.
///
/// You usually don't need to call this manually.
pub fn run_scope_undisposed<T>(
    runtime: RuntimeId,
    f: impl FnOnce(Scope) -> T + 'static,
) -> (T, ScopeId, ScopeDisposer) {
    runtime.run_scope_undisposed(f, None)
}

/// A Each scope can have
/// child scopes, and may in turn have a parent.
///
/// Scopes manage memory within the reactive system. When a scope is disposed, its
/// cleanup functions run and the signals, effects, memos, resources, and contexts
/// associated with it no longer exist and should no longer be accessed.
///
/// You generally won’t need to create your own scopes when writing application code.
/// However, they’re very useful for managing control flow within an application or library.
/// For example, if you are writing a keyed list component, you will want to create a child scope
/// for each row in the list so that you can dispose of its associated signals, etc.
/// when it is removed from the list.
///
/// Every other function in this crate takes a `Scope` as its first argument. Since `Scope`
/// is [Copy] and `'static` this does not add much overhead or lifetime complexity.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Scope {
    #[doc(hidden)]
    pub runtime: RuntimeId,
    #[doc(hidden)]
    pub id: ScopeId,
}

impl Scope {
    /// The unique identifier for this scope.
    pub fn id(&self) -> ScopeId {
        self.id
    }

    /// Creates a child scope and runs the given function within it, returning a handle to dispose of it.
    ///
    /// The child scope has its own lifetime and disposer, but will be disposed when the parent is
    /// disposed, if it has not been already.
    ///
    /// This is useful for applications like a list or a router, which may want to create child scopes and
    /// dispose of them when they are no longer needed (e.g., a list item has been destroyed or the user
    /// has navigated away from the route.)
    pub fn child_scope(self, f: impl FnOnce(Scope)) -> ScopeDisposer {
        let (_, disposer) = self.run_child_scope(f);
        disposer
    }

    /// Creates a child scope and runs the given function within it, returning the function's return
    /// type and a handle to dispose of it.
    ///
    /// The child scope has its own lifetime and disposer, but will be disposed when the parent is
    /// disposed, if it has not been already.
    ///
    /// This is useful for applications like a list or a router, which may want to create child scopes and
    /// dispose of them when they are no longer needed (e.g., a list item has been destroyed or the user
    /// has navigated away from the route.)
    pub fn run_child_scope<T>(self, f: impl FnOnce(Scope) -> T) -> (T, ScopeDisposer) {
        let (res, child_id, disposer) = self.runtime.run_scope_undisposed(f, Some(self));
        with_runtime(self.runtime, |runtime| {
            let mut children = runtime.scope_children.borrow_mut();
            children
                .entry(self.id)
                .expect("trying to add a child to a Scope that has already been disposed")
                .or_default()
                .push(child_id);
        });
        (res, disposer)
    }

    /// Suspends reactive tracking while running the given function.
    ///
    /// This can be used to isolate parts of the reactive graph from one another.
    ///
    /// ```
    /// # use leptos_reactive::*;
    /// # run_scope(create_runtime(), |cx| {
    /// let (a, set_a) = create_signal(cx, 0);
    /// let (b, set_b) = create_signal(cx, 0);
    /// let c = create_memo(cx, move |_| {
    ///     // this memo will *only* update when `a` changes
    ///     a() + cx.untrack(move || b())
    /// });
    ///
    /// assert_eq!(c(), 0);
    /// set_a(1);
    /// assert_eq!(c(), 1);
    /// set_b(1);
    /// // hasn't updated, because we untracked before reading b
    /// assert_eq!(c(), 1);
    /// set_a(2);
    /// assert_eq!(c(), 3);
    ///
    /// # });
    /// ```
    pub fn untrack<T>(&self, f: impl FnOnce() -> T) -> T {
        with_runtime(self.runtime, |runtime| {
            let prev_observer = runtime.observer.take();
            let untracked_result = f();
            runtime.observer.set(prev_observer);
            untracked_result
        })
    }
}

// Internals

impl Scope {
    pub(crate) fn dispose(self) {
        with_runtime(self.runtime, |runtime| {
            // dispose of all child scopes
            let children = {
                let mut children = runtime.scope_children.borrow_mut();
                children.remove(self.id)
            };

            if let Some(children) = children {
                for id in children {
                    Scope {
                        runtime: self.runtime,
                        id,
                    }
                    .dispose();
                }
            }

            // run cleanups
            if let Some(cleanups) = runtime.scope_cleanups.borrow_mut().remove(self.id) {
                for cleanup in cleanups {
                    cleanup();
                }
            }

            // remove everything we own and run cleanups
            let owned = {
                let owned = runtime.scopes.borrow_mut().remove(self.id);
                owned.map(|owned| owned.take())
            };
            if let Some(owned) = owned {
                for property in owned {
                    match property {
                        ScopeProperty::Signal(id) => {
                            // remove the signal
                            runtime.signals.borrow_mut().remove(id);
                            let subs = runtime.signal_subscribers.borrow_mut().remove(id);

                            // each of the subs needs to remove the signal from its dependencies
                            // so that it doesn't try to read the (now disposed) signal
                            if let Some(subs) = subs {
                                let source_map = runtime.effect_sources.borrow();
                                for effect in subs.borrow().iter() {
                                    if let Some(effect_sources) = source_map.get(*effect) {
                                        effect_sources.borrow_mut().remove(&id);
                                    }
                                }
                            }
                        }
                        ScopeProperty::Effect(id) => {
                            runtime.effects.borrow_mut().remove(id);
                            runtime.effect_sources.borrow_mut().remove(id);
                        }
                        ScopeProperty::Resource(id) => {
                            runtime.resources.borrow_mut().remove(id);
                        }
                    }
                }
            }
        })
    }

    pub(crate) fn with_scope_property(&self, f: impl FnOnce(&mut Vec<ScopeProperty>)) {
        with_runtime(self.runtime, |runtime| {
            let scopes = runtime.scopes.borrow();
            let scope = scopes
                .get(self.id)
                .expect("tried to add property to a scope that has been disposed");
            f(&mut scope.borrow_mut());
        })
    }
}

/// Creates a cleanup function, which will be run when a [Scope] is disposed.
///
/// It runs after child scopes have been disposed, but before signals, effects, and resources
/// are invalidated.
pub fn on_cleanup(cx: Scope, cleanup_fn: impl FnOnce() + 'static) {
    with_runtime(cx.runtime, |runtime| {
        let mut cleanups = runtime.scope_cleanups.borrow_mut();
        let cleanups = cleanups
            .entry(cx.id)
            .expect("trying to clean up a Scope that has already been disposed")
            .or_insert_with(Default::default);
        cleanups.push(Box::new(cleanup_fn));
    })
}

slotmap::new_key_type! {
    /// Unique ID assigned to a [Scope](crate::Scope).
    pub struct ScopeId;
}

#[derive(Debug)]
pub(crate) enum ScopeProperty {
    Signal(SignalId),
    Effect(EffectId),
    Resource(ResourceId),
}

/// Creating a [Scope](crate::Scope) gives you a disposer, which can be called
/// to dispose of that reactive scope.
///
/// This will
/// 1. dispose of all child `Scope`s
/// 2. run all cleanup functions defined for this scope by [on_cleanup](crate::on_cleanup).
/// 3. dispose of all signals, effects, and resources owned by this `Scope`.
pub struct ScopeDisposer(pub(crate) Box<dyn FnOnce()>);

impl ScopeDisposer {
    /// Disposes of a reactive [Scope](crate::Scope).
    ///
    /// This will
    /// 1. dispose of all child `Scope`s
    /// 2. run all cleanup functions defined for this scope by [on_cleanup](crate::on_cleanup).
    /// 3. dispose of all signals, effects, and resources owned by this `Scope`.
    pub fn dispose(self) {
        (self.0)()
    }
}

impl Scope {
    /// Returns IDs for all [Resource](crate::Resource)s found on any scope.
    pub fn all_resources(&self) -> Vec<ResourceId> {
        with_runtime(self.runtime, |runtime| runtime.all_resources())
    }

     /// Returns IDs for all [Resource](crate::Resource)s found on any scope that are 
     /// pending from the server.
     pub fn pending_resources(&self) -> Vec<ResourceId> {
        with_runtime(self.runtime, |runtime| runtime.pending_resources())
    }

    /// Returns IDs for all [Resource](crate::Resource)s found on any scope.
    pub fn serialization_resolvers(&self) -> FuturesUnordered<PinnedFuture<(ResourceId, String)>> {
        with_runtime(self.runtime, |runtime| runtime.serialization_resolvers())
    }

    /// Registers the given [SuspenseContext](crate::SuspenseContext) with the current scope,
    /// calling the `resolver` when its resources are all resolved.
    pub fn register_suspense(
        &self,
        context: SuspenseContext,
        key_before_suspense: &str,
        key: &str,
        resolver: impl FnOnce() -> String + 'static,
    ) {
        use crate::create_isomorphic_effect;
        use futures::StreamExt;

        with_runtime(self.runtime, |runtime| {
            let mut shared_context = runtime.shared_context.borrow_mut();
            let (tx, mut rx) = futures::channel::mpsc::unbounded();

            create_isomorphic_effect(*self, move |_| {
                let pending = context.pending_resources.try_with(|n| *n).unwrap_or(0);
                if pending == 0 {
                    _ = tx.unbounded_send(());
                }
            });

            shared_context.pending_fragments.insert(
                key.to_string(),
                (
                    key_before_suspense.to_string(),
                    Box::pin(async move {
                        rx.next().await;
                        resolver()
                    })
                ),
            );
        })
    }

    /// The set of all HTML fragments current pending, by their keys (see [Self::current_fragment_key]).
    /// Returns a tuple of the hydration ID of the previous element, and a pinned `Future` that will yield the
    /// `<Suspense/>` HTML when all resources are resolved.
    pub fn pending_fragments(&self) -> HashMap<String, (String, PinnedFuture<String>)> {
        with_runtime(self.runtime, |runtime| {
            let mut shared_context = runtime.shared_context.borrow_mut();
            std::mem::take(&mut shared_context.pending_fragments)
        })
    }
}

impl fmt::Debug for ScopeDisposer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("ScopeDisposer").finish()
    }
}