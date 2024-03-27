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
        // if update_is_necessary is being called, that mean that a subscriber
        // wants to know if our latest value has changed
        //
        // this could be the case either because
        // 1) we have updated, and asynchronously woken the subscriber back up
        // 2) a different source has woken up the subscriber, and it's now asking us
        //    if we've changed
        //
        // if we return `false` it will short-circuit that subscriber
        // if we return `true` it means "yes, we may have changed"
        //
        // returning `true` here means that an AsyncDerived behaves like a signal (it always says
        // "sure, I"ve changed) and not like a memo (checks whether it has *actually* changed)
        //
        // TODO is there a dirty-checking mechanism that would work here? we would need a
        // memoization process like a memo has, to ensure we don't over-notify
        true
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
