use crate::{
    channel::Sender,
    graph::{
        AnySource, AnySubscriber, ReactiveNode, Source, SourceSet, Subscriber,
        SubscriberSet,
    },
    owner::Owner,
};
use or_poisoned::OrPoisoned;
use std::sync::RwLock;

pub(crate) struct ArcAsyncDerivedInner {
    pub owner: Owner,
    // holds subscribers so the dependency can be cleared when this needs to rerun
    pub sources: SourceSet,
    // tracks reactive subscribers so they can be notified
    // when the new async value is ready
    pub subscribers: SubscriberSet,
    // when a source changes, notifying this will cause the async work to rerun
    pub notifier: Sender,
}

impl ReactiveNode for RwLock<ArcAsyncDerivedInner> {
    fn mark_dirty(&self) {
        self.write().or_poisoned().notifier.notify();
    }

    fn mark_check(&self) {
        self.write().or_poisoned().notifier.notify();
    }

    fn mark_subscribers_check(&self) {
        let lock = self.read().or_poisoned();
        for sub in (&lock.subscribers).into_iter() {
            sub.mark_check();
        }
    }

    fn update_if_necessary(&self) -> bool {
        // always return false, because the async work will not be ready yet
        // we'll mark subscribers dirty again when it resolves
        false
    }
}

impl Source for RwLock<ArcAsyncDerivedInner> {
    fn add_subscriber(&self, subscriber: AnySubscriber) {
        self.write().or_poisoned().subscribers.subscribe(subscriber);
    }

    fn remove_subscriber(&self, subscriber: &AnySubscriber) {
        self.write()
            .or_poisoned()
            .subscribers
            .unsubscribe(subscriber);
    }

    fn clear_subscribers(&self) {
        self.write().or_poisoned().subscribers.take();
    }
}

impl Subscriber for RwLock<ArcAsyncDerivedInner> {
    fn add_source(&self, source: AnySource) {
        self.write().or_poisoned().sources.insert(source);
    }

    fn clear_sources(&self, subscriber: &AnySubscriber) {
        self.write().or_poisoned().sources.clear_sources(subscriber);
    }
}
