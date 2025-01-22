use super::{node::ReactiveNode, AnySource};
#[cfg(debug_assertions)]
use crate::diagnostics::SpecialNonReactiveZone;
use core::{fmt::Debug, hash::Hash};
use std::{cell::RefCell, mem, sync::Weak};

thread_local! {
    static OBSERVER: RefCell<Option<ObserverState>> = const { RefCell::new(None) };
}

#[derive(Debug)]
struct ObserverState {
    subscriber: AnySubscriber,
    untracked: bool,
}

/// The current reactive observer.
///
/// The observer is whatever reactive node is currently listening for signals that need to be
/// tracked. For example, if an effect is running, that effect is the observer, which means it will
/// subscribe to changes in any signals that are read.
pub struct Observer;

#[derive(Debug)]
struct SetObserverOnDrop(Option<AnySubscriber>);

impl Drop for SetObserverOnDrop {
    fn drop(&mut self) {
        Observer::set(self.0.take());
    }
}

impl Observer {
    /// Returns the current observer, if any.
    pub fn get() -> Option<AnySubscriber> {
        OBSERVER.with_borrow(|obs| {
            obs.as_ref().and_then(|obs| {
                if obs.untracked {
                    None
                } else {
                    Some(obs.subscriber.clone())
                }
            })
        })
    }

    pub(crate) fn is(observer: &AnySubscriber) -> bool {
        OBSERVER.with_borrow(|o| {
            o.as_ref().map(|o| &o.subscriber) == Some(observer)
        })
    }

    fn take() -> SetObserverOnDrop {
        SetObserverOnDrop(
            OBSERVER.with_borrow_mut(Option::take).map(|o| o.subscriber),
        )
    }

    fn set(observer: Option<AnySubscriber>) {
        OBSERVER.with_borrow_mut(|o| {
            *o = observer.map(|subscriber| ObserverState {
                subscriber,
                untracked: false,
            })
        });
    }

    fn replace(observer: Option<AnySubscriber>) -> SetObserverOnDrop {
        SetObserverOnDrop(
            OBSERVER
                .with(|o| {
                    mem::replace(
                        &mut *o.borrow_mut(),
                        observer.map(|subscriber| ObserverState {
                            subscriber,
                            untracked: false,
                        }),
                    )
                })
                .map(|o| o.subscriber),
        )
    }

    fn replace_untracked(observer: Option<AnySubscriber>) -> SetObserverOnDrop {
        SetObserverOnDrop(
            OBSERVER
                .with(|o| {
                    mem::replace(
                        &mut *o.borrow_mut(),
                        observer.map(|subscriber| ObserverState {
                            subscriber,
                            untracked: true,
                        }),
                    )
                })
                .map(|o| o.subscriber),
        )
    }
}

/// Suspends reactive tracking while running the given function.
///
/// This can be used to isolate parts of the reactive graph from one another.
///
/// ```rust
/// # use reactive_graph::computed::*;
/// # use reactive_graph::signal::*; let owner = reactive_graph::owner::Owner::new(); owner.set();
/// # use reactive_graph::prelude::*;
/// # use reactive_graph::graph::untrack;
/// # tokio_test::block_on(async move {
/// # any_spawner::Executor::init_tokio(); let owner = reactive_graph::owner::Owner::new(); owner.set();
/// let (a, set_a) = signal(0);
/// let (b, set_b) = signal(0);
/// let c = Memo::new(move |_| {
///     // this memo will *only* update when `a` changes
///     a.get() + untrack(move || b.get())
/// });
///
/// assert_eq!(c.get(), 0);
/// set_a.set(1);
/// assert_eq!(c.get(), 1);
/// set_b.set(1);
/// // hasn't updated, because we untracked before reading b
/// assert_eq!(c.get(), 1);
/// set_a.set(2);
/// assert_eq!(c.get(), 3);
/// # });
/// ```
#[track_caller]
pub fn untrack<T>(fun: impl FnOnce() -> T) -> T {
    #[cfg(debug_assertions)]
    let _warning_guard = crate::diagnostics::SpecialNonReactiveZone::enter();

    let _prev = Observer::take();
    fun()
}

#[doc(hidden)]
#[track_caller]
pub fn untrack_with_diagnostics<T>(fun: impl FnOnce() -> T) -> T {
    let _prev = Observer::take();
    fun()
}

/// Converts a [`Subscriber`] to a type-erased [`AnySubscriber`].
pub trait ToAnySubscriber {
    /// Converts this type to its type-erased equivalent.
    fn to_any_subscriber(&self) -> AnySubscriber;
}

/// Any type that can track reactive values (like an effect or a memo).
pub trait Subscriber: ReactiveNode {
    /// Adds a subscriber to this subscriber's list of dependencies.
    fn add_source(&self, source: AnySource);

    /// Clears the set of sources for this subscriber.
    fn clear_sources(&self, subscriber: &AnySubscriber);
}

/// A type-erased subscriber.
#[derive(Clone)]
pub struct AnySubscriber(pub usize, pub Weak<dyn Subscriber + Send + Sync>);

impl ToAnySubscriber for AnySubscriber {
    fn to_any_subscriber(&self) -> AnySubscriber {
        self.clone()
    }
}

impl Subscriber for AnySubscriber {
    fn add_source(&self, source: AnySource) {
        if let Some(inner) = self.1.upgrade() {
            inner.add_source(source);
        }
    }

    fn clear_sources(&self, subscriber: &AnySubscriber) {
        if let Some(inner) = self.1.upgrade() {
            inner.clear_sources(subscriber);
        }
    }
}

impl ReactiveNode for AnySubscriber {
    fn mark_dirty(&self) {
        if let Some(inner) = self.1.upgrade() {
            inner.mark_dirty()
        }
    }

    fn mark_subscribers_check(&self) {
        if let Some(inner) = self.1.upgrade() {
            inner.mark_subscribers_check()
        }
    }

    fn update_if_necessary(&self) -> bool {
        if let Some(inner) = self.1.upgrade() {
            inner.update_if_necessary()
        } else {
            false
        }
    }

    fn mark_check(&self) {
        if let Some(inner) = self.1.upgrade() {
            inner.mark_check()
        }
    }
}

/// Runs code with some subscriber as the thread-local [`Observer`].
pub trait WithObserver {
    /// Runs the given function with this subscriber as the thread-local [`Observer`].
    fn with_observer<T>(&self, fun: impl FnOnce() -> T) -> T;

    /// Runs the given function with this subscriber as the thread-local [`Observer`],
    /// but without tracking dependencies.
    fn with_observer_untracked<T>(&self, fun: impl FnOnce() -> T) -> T;
}

impl WithObserver for AnySubscriber {
    fn with_observer<T>(&self, fun: impl FnOnce() -> T) -> T {
        let _prev = Observer::replace(Some(self.clone()));
        fun()
    }

    fn with_observer_untracked<T>(&self, fun: impl FnOnce() -> T) -> T {
        #[cfg(debug_assertions)]
        let _guard = SpecialNonReactiveZone::enter();
        let _prev = Observer::replace_untracked(Some(self.clone()));
        fun()
    }
}

impl WithObserver for Option<AnySubscriber> {
    fn with_observer<T>(&self, fun: impl FnOnce() -> T) -> T {
        let _prev = Observer::replace(self.clone());
        fun()
    }

    fn with_observer_untracked<T>(&self, fun: impl FnOnce() -> T) -> T {
        #[cfg(debug_assertions)]
        let _guard = SpecialNonReactiveZone::enter();
        let _prev = Observer::replace_untracked(self.clone());
        fun()
    }
}

impl Debug for AnySubscriber {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("AnySubscriber").field(&self.0).finish()
    }
}

impl Hash for AnySubscriber {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

impl PartialEq for AnySubscriber {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Eq for AnySubscriber {}
