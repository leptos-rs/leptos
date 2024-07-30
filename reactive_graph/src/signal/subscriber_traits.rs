//! Traits to reduce the boilerplate when implementing the [`ReactiveNode`], [`Source`], and
//! [`ToAnySource`] traits for signal types.
//!
//! These traits can be automatically derived for any type that
//! 1) is a root node in the reactive graph, with no sources (i.e., a signal, not a memo)
//! 2) contains an `Arc<RwLock<SubscriberSet>>`
//!
//! This makes it easy to implement a variety of different signal primitives, as long as they share
//! these characteristics.

use crate::{
    graph::{
        AnySource, AnySubscriber, ReactiveNode, Source, SubscriberSet,
        ToAnySource,
    },
    traits::DefinedAt,
    unwrap_signal,
};
use or_poisoned::OrPoisoned;
use std::{
    borrow::Borrow,
    sync::{Arc, RwLock, Weak},
};

pub(crate) trait AsSubscriberSet {
    type Output: Borrow<RwLock<SubscriberSet>>;

    fn as_subscriber_set(&self) -> Option<Self::Output>;
}

pub(crate) trait ToSubscriberSet {
    fn to_subscriber_set(&self) -> RwLock<SubscriberSet>;
}

impl<'a> AsSubscriberSet for &'a RwLock<SubscriberSet> {
    type Output = &'a RwLock<SubscriberSet>;

    #[inline(always)]
    fn as_subscriber_set(&self) -> Option<Self::Output> {
        Some(self)
    }
}

impl DefinedAt for RwLock<SubscriberSet> {
    fn defined_at(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }
}

// Implement reactive types for RwLock<SubscriberSet>
// This is used so that Weak<RwLock<SubscriberSet>> is a Weak<dyn ReactiveNode> and Weak<dyn
// Source>
impl<T: AsSubscriberSet + DefinedAt> ReactiveNode for T {
    fn mark_dirty(&self) {
        self.mark_subscribers_check();
    }

    fn mark_check(&self) {}

    fn mark_subscribers_check(&self) {
        if let Some(inner) = self.as_subscriber_set() {
            let subs = inner.borrow().write().unwrap().take();
            for sub in subs {
                sub.mark_check();
            }
        }
    }

    fn update_if_necessary(&self) -> bool {
        // if they're being checked, signals always count as "dirty"
        true
    }
}

impl<T: AsSubscriberSet + DefinedAt> Source for T {
    fn clear_subscribers(&self) {
        if let Some(inner) = self.as_subscriber_set() {
            inner.borrow().write().unwrap().take();
        }
    }

    fn add_subscriber(&self, subscriber: AnySubscriber) {
        if let Some(inner) = self.as_subscriber_set() {
            inner.borrow().write().unwrap().subscribe(subscriber)
        }
    }

    fn remove_subscriber(&self, subscriber: &AnySubscriber) {
        if let Some(inner) = self.as_subscriber_set() {
            inner.borrow().write().unwrap().unsubscribe(subscriber)
        }
    }
}

impl<T: AsSubscriberSet + DefinedAt> ToAnySource for T
where
    T::Output: Borrow<Arc<RwLock<SubscriberSet>>>,
{
    fn to_any_source(&self) -> AnySource {
        self.as_subscriber_set()
            .map(|subs| {
                let subs = subs.borrow();
                AnySource(
                    Arc::as_ptr(subs) as usize,
                    Arc::downgrade(subs) as Weak<dyn Source + Send + Sync>,
                )
            })
            .unwrap_or_else(unwrap_signal!(self))
    }
}

impl ReactiveNode for RwLock<SubscriberSet> {
    fn mark_dirty(&self) {
        self.mark_subscribers_check();
    }

    fn mark_check(&self) {}

    fn mark_subscribers_check(&self) {
        let subs = self.write().unwrap().take();
        for sub in subs {
            sub.mark_check();
        }
    }

    fn update_if_necessary(&self) -> bool {
        // if they're being checked, signals always count as "dirty"
        true
    }
}

impl Source for RwLock<SubscriberSet> {
    fn clear_subscribers(&self) {
        self.write().or_poisoned().take();
    }

    fn add_subscriber(&self, subscriber: AnySubscriber) {
        self.write().or_poisoned().subscribe(subscriber)
    }

    fn remove_subscriber(&self, subscriber: &AnySubscriber) {
        self.write().or_poisoned().unsubscribe(subscriber)
    }
}
