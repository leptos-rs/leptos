use super::suspense::SharedTaskHandle;
use crate::{
    channel::Sender,
    computed::suspense::SuspenseContext,
    graph::{
        AnySource, AnySubscriber, ReactiveNode, Source, SourceSet, Subscriber,
        SubscriberSet,
    },
    owner::Owner,
    transition::AsyncTransition,
};
use futures::channel::oneshot;
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
    pub state: AsyncDerivedState,
    pub version: usize,
    pub suspenses: Vec<SuspenseContext>,
    pub pending_suspenses: Vec<SharedTaskHandle>,
    // completes the loads registered with the `AsyncTransition`s that were
    // active when sources notified this node; the worker takes the whole
    // batch when it handles the notifications, and completes it once the
    // reload they cause has finished (or drops it if they cause none)
    pub transitions: Vec<oneshot::Sender<()>>,
    // whether a worker task will handle notifications at all: a node without
    // one (e.g. a server-side mock) must not register loads with a transition
    // that nothing would ever complete
    pub has_worker: bool,
}

impl ArcAsyncDerivedInner {
    fn register_with_transition(&mut self) {
        if self.has_worker
            && let Some(tx) = AsyncTransition::register_pending()
        {
            self.transitions.push(tx);
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum AsyncDerivedState {
    Clean,
    Dirty,
    Notifying,
}

impl ReactiveNode for RwLock<ArcAsyncDerivedInner> {
    fn mark_dirty(&self) {
        let mut lock = self.write().or_poisoned();
        if lock.state != AsyncDerivedState::Notifying {
            lock.state = AsyncDerivedState::Dirty;
            lock.register_with_transition();
            lock.notifier.notify();
        }
    }

    fn mark_check(&self) {
        let mut lock = self.write().or_poisoned();
        if lock.state != AsyncDerivedState::Notifying {
            lock.register_with_transition();
            lock.notifier.notify();
        }
    }

    fn mark_subscribers_check(&self) {
        let subs = self.read().or_poisoned().subscribers.clone();
        for sub in subs {
            sub.mark_check();
        }
    }

    fn update_if_necessary(&self) -> bool {
        let mut guard = self.write().or_poisoned();
        let (is_dirty, sources) = (
            guard.state == AsyncDerivedState::Dirty,
            (guard.state != AsyncDerivedState::Notifying)
                .then(|| guard.sources.clone()),
        );

        if is_dirty {
            guard.state = AsyncDerivedState::Clean;
            return true;
        }
        drop(guard);

        for source in sources.into_iter().flatten() {
            if source.update_if_necessary() {
                return true;
            }
        }
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
