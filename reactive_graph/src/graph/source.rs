use super::{node::ReactiveNode, AnySubscriber};
use crate::traits::{DefinedAt, IsDisposed};
use core::{fmt::Debug, hash::Hash};
use std::{panic::Location, sync::Weak};

/// Abstracts over the type of any reactive source.
pub trait ToAnySource: IsDisposed {
    /// Converts this type to its type-erased equivalent.
    fn to_any_source(&self) -> AnySource;
}

/// Describes the behavior of any source of reactivity (like a signal, trigger, or memo.)
pub trait Source: ReactiveNode {
    /// Adds a subscriber to this source's list of dependencies.
    fn add_subscriber(&self, subscriber: AnySubscriber);

    /// Removes a subscriber from this source's list of dependencies.
    fn remove_subscriber(&self, subscriber: &AnySubscriber);

    /// Remove all subscribers from this source's list of dependencies.
    fn clear_subscribers(&self);
}

/// A weak reference to any reactive source node.
#[derive(Clone)]
pub struct AnySource(
    pub(crate) usize,
    pub(crate) Weak<dyn Source + Send + Sync>,
    #[cfg(any(debug_assertions, leptos_debuginfo))]
    pub(crate)  &'static Location<'static>,
);

impl DefinedAt for AnySource {
    fn defined_at(&self) -> Option<&'static Location<'static>> {
        #[cfg(any(debug_assertions, leptos_debuginfo))]
        {
            Some(self.2)
        }
        #[cfg(not(any(debug_assertions, leptos_debuginfo)))]
        {
            None
        }
    }
}

impl Debug for AnySource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("AnySource").field(&self.0).finish()
    }
}

impl Hash for AnySource {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

impl PartialEq for AnySource {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Eq for AnySource {}

impl IsDisposed for AnySource {
    #[inline(always)]
    fn is_disposed(&self) -> bool {
        false
    }
}

impl ToAnySource for AnySource {
    fn to_any_source(&self) -> AnySource {
        self.clone()
    }
}

impl Source for AnySource {
    fn add_subscriber(&self, subscriber: AnySubscriber) {
        if let Some(inner) = self.1.upgrade() {
            inner.add_subscriber(subscriber)
        }
    }

    fn remove_subscriber(&self, subscriber: &AnySubscriber) {
        if let Some(inner) = self.1.upgrade() {
            inner.remove_subscriber(subscriber)
        }
    }

    fn clear_subscribers(&self) {
        if let Some(inner) = self.1.upgrade() {
            inner.clear_subscribers();
        }
    }
}

impl ReactiveNode for AnySource {
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
