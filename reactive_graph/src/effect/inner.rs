use crate::{
    channel::Sender,
    graph::{
        AnySource, AnySubscriber, ReactiveNode, SourceSet, Subscriber,
        ToAnySubscriber,
    },
};
use or_poisoned::OrPoisoned;
use std::sync::{Arc, RwLock, Weak};

pub(crate) struct EffectInner {
    pub observer: Sender,
    pub sources: SourceSet,
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

    // TODO check if this actually works for memos
    fn update_if_necessary(&self) -> bool {
        let sources = {
            let guard = self.read().or_poisoned();
            guard.sources.clone()
        };

        for source in sources {
            if source.update_if_necessary() {
                return true;
            }
        }
        false
    }

    fn mark_check(&self) {
        self.write().or_poisoned().observer.notify()
    }

    fn mark_dirty(&self) {
        self.write().or_poisoned().observer.notify()
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
