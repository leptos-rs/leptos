use crate::{
    graph::{
        AnySource, AnySubscriber, Observer, ReactiveNode, Source, SourceSet,
        Subscriber, SubscriberSet, WithObserver,
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
    /// Must always be acquired *after* the reactivity lock
    pub(crate) value: Arc<RwLock<Option<S::Wrapped>>>,
    #[allow(clippy::type_complexity)]
    pub(crate) fun: Arc<dyn Fn(Option<T>) -> (T, bool) + Send + Sync>,
    pub(crate) owner: Owner,
    pub(crate) reactivity: RwLock<MemoInnerReactivity>,
}

/// The current state of a memo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MemoState {
    /// The value is up to date.
    Clean,
    /// A source may have changed; check sources before using the value.
    Check,
    /// A source has definitely changed; recompute before using the value.
    Dirty,
    /// Checking whether any source has changed. The value is present.
    Checking,
    /// Recomputing the value. The value has been taken to pass to the memo function.
    Running,
    /// Recomputing the value, and has been marked "check" since it began recomputing.
    /// After running, the memo will be marked `Check`.
    RunningCheck,
    /// Recomputing the value, and has been marked "dirty" since it began recomputing.
    /// After running, the memo will be marked `Dirty`.
    RunningDirty,
}

impl MemoState {
    fn is_running(self) -> bool {
        matches!(
            self,
            MemoState::Running
                | MemoState::RunningCheck
                | MemoState::RunningDirty
        )
    }
}

pub(crate) struct MemoInnerReactivity {
    pub(crate) state: MemoState,
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
                state: MemoState::Dirty,
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
        let subs = {
            let mut lock = self.reactivity.write().or_poisoned();
            lock.state = if lock.state.is_running() {
                MemoState::RunningDirty
            } else {
                MemoState::Dirty
            };
            lock.subscribers.clone()
        };

        for sub in subs {
            sub.mark_check();
        }
    }

    fn mark_check(&self) {
        /// codegen optimisation:
        fn inner(reactivity: &RwLock<MemoInnerReactivity>) {
            let subs = {
                let mut lock = reactivity.write().or_poisoned();
                lock.state = match lock.state {
                    MemoState::Running | MemoState::RunningCheck => {
                        MemoState::RunningCheck
                    }
                    MemoState::RunningDirty => MemoState::RunningDirty,
                    MemoState::Dirty => MemoState::Dirty,
                    MemoState::Clean
                    | MemoState::Check
                    | MemoState::Checking => MemoState::Check,
                };
                lock.subscribers.clone()
            };

            for sub in subs {
                sub.mark_check();
            }
        }
        inner(&self.reactivity);
    }

    fn mark_subscribers_check(&self) {
        let subs = self.reactivity.read().or_poisoned().subscribers.clone();
        for sub in subs {
            sub.mark_check();
        }
    }

    fn update_if_necessary(&self) -> bool {
        /// codegen optimisation:
        fn needs_update(reactivity: &RwLock<MemoInnerReactivity>) -> bool {
            let sources = {
                let mut inner = reactivity.write().or_poisoned();
                match inner.state {
                    MemoState::Clean => return false,
                    // While another thread is running, the value is absent,
                    // so this thread has to run too.
                    MemoState::Dirty
                    | MemoState::Running
                    | MemoState::RunningCheck
                    | MemoState::RunningDirty => return true,
                    MemoState::Check | MemoState::Checking => {
                        inner.state = MemoState::Checking;
                        inner.sources.clone()
                    }
                }
            };
            let changed = (&sources).into_iter().any(|source| {
                source.update_if_necessary()
                    || reactivity.read().or_poisoned().state == MemoState::Dirty
            });
            if !changed {
                // No source changed. Only mark clean if no mark arrived
                // during the check; otherwise leave it for the next read.
                let mut inner = reactivity.write().or_poisoned();
                if inner.state == MemoState::Checking {
                    inner.state = MemoState::Clean;
                }
            }
            changed
        }

        if needs_update(&self.reactivity) {
            // No deadlock risk, because we only hold the value lock.
            let value = self.value.write().or_poisoned().take();

            /// codegen optimisation:
            fn inner_1(
                reactivity: &RwLock<MemoInnerReactivity>,
            ) -> AnySubscriber {
                let any_subscriber = {
                    let mut lock = reactivity.write().or_poisoned();

                    if !lock.state.is_running() {
                        lock.state = MemoState::Running;
                    }
                    lock.any_subscriber.clone()
                };
                any_subscriber.clear_sources(&any_subscriber);
                any_subscriber
            }
            let any_subscriber = inner_1(&self.reactivity);

            let (new_value, changed) = self.owner.with_cleanup(|| {
                any_subscriber.with_observer(|| {
                    (self.fun)(value.map(StorageAccess::into_taken))
                })
            });

            // Two locks are acquired, so order matters.
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
                reactivity_lock.state = match reactivity_lock.state {
                    MemoState::Running => MemoState::Clean,
                    MemoState::RunningCheck => MemoState::Check,
                    MemoState::RunningDirty => MemoState::Dirty,
                    other => other,
                };

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
            false
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
