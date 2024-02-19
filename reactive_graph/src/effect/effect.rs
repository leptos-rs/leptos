use crate::{
    channel::{channel, Receiver},
    effect::inner::EffectInner,
    graph::{AnySubscriber, SourceSet, Subscriber, ToAnySubscriber},
    owner::Owner,
};
use any_spawner::Executor;
use futures::StreamExt;
use or_poisoned::OrPoisoned;
use std::{
    mem,
    sync::{Arc, RwLock},
};

pub struct Effect<T>
where
    T: 'static,
{
    value: Arc<RwLock<Option<T>>>,
    inner: Arc<RwLock<EffectInner>>,
}

impl<T> Clone for Effect<T> {
    fn clone(&self) -> Self {
        Self {
            value: Arc::clone(&self.value),
            inner: Arc::clone(&self.inner),
        }
    }
}

fn effect_base() -> (Receiver, Owner, Arc<RwLock<EffectInner>>) {
    let (mut observer, rx) = channel();

    // spawn the effect asynchronously
    // we'll notify once so it runs on the next tick,
    // to register observed values
    observer.notify();

    let owner = Owner::new();
    let inner = Arc::new(RwLock::new(EffectInner {
        observer,
        sources: SourceSet::new(),
    }));

    (rx, owner, inner)
}

impl<T> Effect<T>
where
    T: 'static,
{
    pub fn with_value_mut<U>(
        &self,
        fun: impl FnOnce(&mut T) -> U,
    ) -> Option<U> {
        self.value.write().or_poisoned().as_mut().map(fun)
    }

    pub fn new(mut fun: impl FnMut(Option<T>) -> T + 'static) -> Self {
        let (mut rx, owner, inner) = effect_base();
        let value = Arc::new(RwLock::new(None));

        Executor::spawn_local({
            let value = Arc::clone(&value);
            let subscriber = inner.to_any_subscriber();

            async move {
                while rx.next().await.is_some() {
                    subscriber.clear_sources(&subscriber);

                    let old_value =
                        mem::take(&mut *value.write().or_poisoned());
                    let new_value = owner.with_cleanup(|| {
                        subscriber.with_observer(|| fun(old_value))
                    });
                    *value.write().or_poisoned() = Some(new_value);
                }
            }
        });

        Self { value, inner }
    }
}

impl<T> Effect<T>
where
    T: Send + Sync + 'static,
{
    pub fn new_sync(
        mut fun: impl FnMut(Option<T>) -> T + Send + Sync + 'static,
    ) -> Self {
        let (mut rx, owner, inner) = effect_base();
        let value = Arc::new(RwLock::new(None));

        Executor::spawn({
            let value = Arc::clone(&value);
            let subscriber = inner.to_any_subscriber();

            async move {
                while rx.next().await.is_some() {
                    subscriber.clear_sources(&subscriber);

                    let old_value =
                        mem::take(&mut *value.write().or_poisoned());
                    let new_value = owner.with_cleanup(|| {
                        subscriber.with_observer(|| fun(old_value))
                    });
                    *value.write().or_poisoned() = Some(new_value);
                }
            }
        });
        Self { value, inner }
    }
}

impl<T> ToAnySubscriber for Effect<T> {
    fn to_any_subscriber(&self) -> AnySubscriber {
        self.inner.to_any_subscriber()
    }
}
