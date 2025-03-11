use crate::{
    channel::Sender,
    graph::{
        AnySource, AnySubscriber, ReactiveNode, SourceSet, Subscriber,
        ToAnySubscriber,
    },
};
use or_poisoned::OrPoisoned;
use std::sync::{Arc, RwLock, Weak};

/// Handles internal subscription logic for effects.
#[derive(Debug)]
pub struct EffectInner {
    pub(crate) dirty: bool,
    pub(crate) observer: Sender,
    pub(crate) sources: SourceSet,
}

impl ToAnySubscriber for Arc<RwLock<EffectInner>> {
    fn to_any_subscriber(&self) -> AnySubscriber {
        AnySubscriber(
            Arc::as_ptr(self) as usize,
            Arc::downgrade(self) as Weak<dyn Subscriber + Send + Sync>,
        )
    }
}

impl ReactiveNode for RwLock<EffectInner> {
    fn mark_subscribers_check(&self) {}

    fn update_if_necessary(&self) -> bool {
        let mut guard = self.write().or_poisoned();

        if guard.dirty {
            guard.dirty = false;
            return true;
        }

        let sources = guard.sources.clone();

        drop(guard);

        sources
            .into_iter()
            .any(|source| source.update_if_necessary())
    }

    fn mark_check(&self) {
        self.write().or_poisoned().observer.notify()
    }

    fn mark_dirty(&self) {
        let mut lock = self.write().or_poisoned();
        lock.dirty = true;
        lock.observer.notify()
    }
}

impl Subscriber for RwLock<EffectInner> {
    fn add_source(&self, source: AnySource) {
        self.write().or_poisoned().sources.insert(source);
    }

    fn clear_sources(&self, subscriber: &AnySubscriber) {
        self.write().or_poisoned().sources.clear_sources(subscriber);
    }
}
