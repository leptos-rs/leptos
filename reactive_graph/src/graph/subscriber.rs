use super::{node::ReactiveNode, AnySource};
use core::{fmt::Debug, hash::Hash};
use std::{cell::RefCell, mem, sync::Weak};

thread_local! {
    static OBSERVER: RefCell<Option<AnySubscriber>> = const { RefCell::new(None) };
}

pub struct Observer;

impl Observer {
    pub fn get() -> Option<AnySubscriber> {
        OBSERVER.with(|o| o.borrow().clone())
    }

    pub(crate) fn is(observer: &AnySubscriber) -> bool {
        OBSERVER.with(|o| o.borrow().as_ref() == Some(observer))
    }

    fn take() -> Option<AnySubscriber> {
        OBSERVER.with(|o| o.borrow_mut().take())
    }

    fn set(observer: Option<AnySubscriber>) {
        OBSERVER.with(|o| *o.borrow_mut() = observer);
    }

    fn replace(observer: AnySubscriber) -> Option<AnySubscriber> {
        OBSERVER.with(|o| mem::replace(&mut *o.borrow_mut(), Some(observer)))
    }
}

pub fn untrack<T>(fun: impl FnOnce() -> T) -> T {
    let prev = Observer::take();
    let value = fun();
    Observer::set(prev);
    value
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

    // Clears the set of sources for this subscriber.
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

impl AnySubscriber {
    pub fn with_observer<T>(&self, fun: impl FnOnce() -> T) -> T {
        let prev = Observer::replace(self.clone());
        let val = fun();
        Observer::set(prev);
        val
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
