//! Types that hold the set of sources or subscribers affiliated with a reactive node.
//!
//! At the moment, these are implemented as linear maps built on a `Vec<_>`. This is for the sake
//! of minimizing binary size as much as possible, and on the assumption that the M:N relationship
//! between sources and subscribers usually consists of fairly small numbers, such that the cost of
//! a linear search is not significantly more expensive than a hash and lookup.

use super::{AnySource, AnySubscriber, Source};
use core::slice;
use std::{mem, vec::IntoIter};

#[derive(Default, Clone, Debug)]
pub struct SourceSet(Vec<AnySource>);

impl SourceSet {
    pub fn new() -> Self {
        Self(Default::default())
    }

    pub fn insert(&mut self, source: AnySource) {
        self.0.push(source);
    }

    pub fn remove(&mut self, source: &AnySource) {
        if let Some(pos) = self.0.iter().position(|s| s == source) {
            self.0.remove(pos);
        }
    }

    pub fn take(&mut self) -> Vec<AnySource> {
        mem::take(&mut self.0)
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn clear_sources(&mut self, subscriber: &AnySubscriber) {
        for source in self.take() {
            source.remove_subscriber(subscriber);
        }
    }
}

impl IntoIterator for SourceSet {
    type Item = AnySource;
    type IntoIter = IntoIter<AnySource>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a> IntoIterator for &'a SourceSet {
    type Item = &'a AnySource;
    type IntoIter = slice::Iter<'a, AnySource>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}
#[derive(Debug, Default, Clone)]
pub struct SubscriberSet(Vec<AnySubscriber>);

impl SubscriberSet {
    pub fn new() -> Self {
        Self(Vec::with_capacity(2))
    }

    pub fn subscribe(&mut self, subscriber: AnySubscriber) {
        if !self.0.contains(&subscriber) {
            self.0.push(subscriber);
        }
    }

    pub fn unsubscribe(&mut self, subscriber: &AnySubscriber) {
        if let Some(pos) = self.0.iter().position(|s| s == subscriber) {
            // note: do not use `.swap_remove()` here.
            // using `.remove()` is slower because it shifts other items
            // but it maintains the order of the subscribers, which is important
            // to correctness when you're using this to drive something like a UI,
            // which can have nested effects, where the inner one assumes the outer
            // has already run (for example, an outer effect that checks .is_some(),
            // and an inner effect that unwraps)
            self.0.remove(pos);
        }
    }

    pub fn take(&mut self) -> Vec<AnySubscriber> {
        mem::take(&mut self.0)
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }
}

impl IntoIterator for SubscriberSet {
    type Item = AnySubscriber;
    type IntoIter = IntoIter<AnySubscriber>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a> IntoIterator for &'a SubscriberSet {
    type Item = &'a AnySubscriber;
    type IntoIter = slice::Iter<'a, AnySubscriber>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}
