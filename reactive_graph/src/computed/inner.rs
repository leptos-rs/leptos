use crate::{
    graph::{
        AnySource, AnySubscriber, Observer, ReactiveNode, ReactiveNodeState,
        Source, SourceSet, Subscriber, SubscriberSet, WithObserver,
    },
    owner::{Owner, Storage, StorageAccess},
};
use or_poisoned::OrPoisoned;
use std::{
    fmt::Debug,
    sync::{Arc, RwLock, RwLockWriteGuard},
};

pub struct MemoInner<T, S>
where
    S: Storage<T>,
{
    /// Must always be aquired *after* the reactivity lock
    pub(crate) value: Arc<RwLock<Option<S::Wrapped>>>,
    #[allow(clippy::type_complexity)]
    pub(crate) fun: Arc<dyn Fn(Option<T>) -> (T, bool) + Send + Sync>,
    pub(crate) owner: Owner,
    pub(crate) reactivity: RwLock<MemoInnerReactivity>,
}

pub(crate) struct MemoInnerReactivity {
    pub(crate) state: ReactiveNodeState,
    pub(crate) sources: SourceSet,
    pub(crate) subscribers: SubscriberSet,
    pub(crate) any_subscriber: AnySubscriber,
}

impl<T, S> Debug for MemoInner<T, S>
where
    S: Storage<T>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MemoInner").finish_non_exhaustive()
    }
}

impl<T: 'static, S> MemoInner<T, S>
where
    S: Storage<T>,
{
    #[allow(clippy::type_complexity)]
    pub fn new(
        fun: Arc<dyn Fn(Option<T>) -> (T, bool) + Send + Sync>,
        any_subscriber: AnySubscriber,
    ) -> Self {
        Self {
            value: Arc::new(RwLock::new(None)),
            fun,
            owner: Owner::new(),
            reactivity: RwLock::new(MemoInnerReactivity {
                state: ReactiveNodeState::Dirty,
                sources: Default::default(),
                subscribers: SubscriberSet::new(),
                any_subscriber,
            }),
        }
    }
}

impl<T: 'static, S> ReactiveNode for MemoInner<T, S>
where
    S: Storage<T>,
{
    fn mark_dirty(&self) {
        self.reactivity.write().or_poisoned().state = ReactiveNodeState::Dirty;
        self.mark_subscribers_check();
    }

    fn mark_check(&self) {
        /// codegen optimisation:
        fn inner(reactivity: &RwLock<MemoInnerReactivity>) {
            {
                let mut lock = reactivity.write().or_poisoned();
                if lock.state != ReactiveNodeState::Dirty {
                    lock.state = ReactiveNodeState::Check;
                }
            }
            for sub in
                (&reactivity.read().or_poisoned().subscribers).into_iter()
            {
                sub.mark_check();
            }
        }
        inner(&self.reactivity);
    }

    fn mark_subscribers_check(&self) {
        let lock = self.reactivity.read().or_poisoned();
        for sub in (&lock.subscribers).into_iter() {
            sub.mark_check();
        }
    }

    fn update_if_necessary(&self) -> bool {
        /// codegen optimisation:
        fn needs_update(reactivity: &RwLock<MemoInnerReactivity>) -> bool {
            let (state, sources) = {
                let inner = reactivity.read().or_poisoned();
                (inner.state, inner.sources.clone())
            };
            match state {
                ReactiveNodeState::Clean => false,
                ReactiveNodeState::Dirty => true,
                ReactiveNodeState::Check => {
                    (&sources).into_iter().any(|source| {
                        source.update_if_necessary()
                            || reactivity.read().or_poisoned().state
                                == ReactiveNodeState::Dirty
                    })
                }
            }
        }

        if needs_update(&self.reactivity) {
            // No deadlock risk, because we only hold the value lock.
            let value = self.value.write().or_poisoned().take();

            /// codegen optimisation:
            fn inner_1(
                reactivity: &RwLock<MemoInnerReactivity>,
            ) -> AnySubscriber {
                let any_subscriber =
                    reactivity.read().or_poisoned().any_subscriber.clone();
                any_subscriber.clear_sources(&any_subscriber);
                any_subscriber
            }
            let any_subscriber = inner_1(&self.reactivity);

            let (new_value, changed) = self.owner.with_cleanup(|| {
                any_subscriber.with_observer(|| {
                    (self.fun)(value.map(StorageAccess::into_taken))
                })
            });

            // Two locks are aquired, so order matters.
            let reactivity_lock = self.reactivity.write().or_poisoned();
            {
                // Safety: Can block endlessly if the user is has a ReadGuard on the value
                let mut value_lock = self.value.write().or_poisoned();
                *value_lock = Some(S::wrap(new_value));
            }

            /// codegen optimisation:
            fn inner_2(
                changed: bool,
                mut reactivity_lock: RwLockWriteGuard<'_, MemoInnerReactivity>,
            ) {
                reactivity_lock.state = ReactiveNodeState::Clean;

                if changed {
                    let subs = reactivity_lock.subscribers.clone();
                    drop(reactivity_lock);
                    for sub in subs {
                        // don't trigger reruns of effects/memos
                        // basically: if one of the observers has triggered this memo to
                        // run, it doesn't need to be re-triggered because of this change
                        if !Observer::is(&sub) {
                            sub.mark_dirty();
                        }
                    }
                } else {
                    drop(reactivity_lock);
                }
            }
            inner_2(changed, reactivity_lock);

            changed
        } else {
            /// codegen optimisation:
            fn inner(reactivity: &RwLock<MemoInnerReactivity>) -> bool {
                let mut lock = reactivity.write().or_poisoned();
                lock.state = ReactiveNodeState::Clean;
                false
            }
            inner(&self.reactivity)
        }
    }
}

impl<T: 'static, S> Source for MemoInner<T, S>
where
    S: Storage<T>,
{
    fn add_subscriber(&self, subscriber: AnySubscriber) {
        let mut lock = self.reactivity.write().or_poisoned();
        lock.subscribers.subscribe(subscriber);
    }

    fn remove_subscriber(&self, subscriber: &AnySubscriber) {
        self.reactivity
            .write()
            .or_poisoned()
            .subscribers
            .unsubscribe(subscriber);
    }

    fn clear_subscribers(&self) {
        self.reactivity.write().or_poisoned().subscribers.take();
    }
}

impl<T: 'static, S> Subscriber for MemoInner<T, S>
where
    S: Storage<T>,
{
    fn add_source(&self, source: AnySource) {
        self.reactivity.write().or_poisoned().sources.insert(source);
    }

    fn clear_sources(&self, subscriber: &AnySubscriber) {
        self.reactivity
            .write()
            .or_poisoned()
            .sources
            .clear_sources(subscriber);
    }
}
