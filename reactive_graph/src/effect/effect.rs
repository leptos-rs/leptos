use crate::{
    channel::{channel, Receiver},
    effect::inner::EffectInner,
    graph::{
        AnySubscriber, ReactiveNode, SourceSet, Subscriber, ToAnySubscriber,
    },
    owner::{Owner, StoredValue},
    traits::Dispose,
};
use any_spawner::Executor;
use futures::StreamExt;
use or_poisoned::OrPoisoned;
use std::{
    mem,
    sync::{Arc, RwLock},
};

pub struct Effect {
    inner: StoredValue<Option<Arc<RwLock<EffectInner>>>>,
}

impl Dispose for Effect {
    fn dispose(self) {
        self.inner.dispose()
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
        dirty: true,
        observer,
        sources: SourceSet::new(),
    }));

    (rx, owner, inner)
}

impl Effect {
    pub fn stop(self) {
        drop(self.inner.try_update_value(|inner| inner.take()));
    }

    pub fn new<T>(mut fun: impl FnMut(Option<T>) -> T + 'static) -> Self
    where
        T: 'static,
    {
        let (mut rx, owner, inner) = effect_base();
        let value = Arc::new(RwLock::new(None::<T>));
        let mut first_run = true;

        #[cfg(feature = "effects")]
        Executor::spawn_local({
            let value = Arc::clone(&value);
            let subscriber = inner.to_any_subscriber();

            async move {
                while rx.next().await.is_some() {
                    if first_run
                        || subscriber
                            .with_observer(|| subscriber.update_if_necessary())
                    {
                        first_run = false;
                        subscriber.clear_sources(&subscriber);

                        let old_value =
                            mem::take(&mut *value.write().or_poisoned());
                        let new_value = owner.with_cleanup(|| {
                            subscriber.with_observer(|| fun(old_value))
                        });
                        *value.write().or_poisoned() = Some(new_value);
                    }
                }
            }
        });

        Self {
            inner: StoredValue::new(Some(inner)),
        }
    }

    pub fn new_sync<T>(
        mut fun: impl FnMut(Option<T>) -> T + Send + Sync + 'static,
    ) -> Self
    where
        T: Send + Sync + 'static,
    {
        let (mut rx, owner, inner) = effect_base();
        let mut first_run = true;
        let value = Arc::new(RwLock::new(None::<T>));

        #[cfg(feature = "effects")]
        Executor::spawn({
            let value = Arc::clone(&value);
            let subscriber = inner.to_any_subscriber();

            async move {
                while rx.next().await.is_some() {
                    if first_run
                        || subscriber
                            .with_observer(|| subscriber.update_if_necessary())
                    {
                        first_run = false;
                        subscriber.clear_sources(&subscriber);

                        let old_value =
                            mem::take(&mut *value.write().or_poisoned());
                        let new_value = owner.with_cleanup(|| {
                            subscriber.with_observer(|| fun(old_value))
                        });
                        *value.write().or_poisoned() = Some(new_value);
                    }
                }
            }
        });
        Self {
            inner: StoredValue::new(Some(inner)),
        }
    }

    pub fn new_isomorphic<T>(
        mut fun: impl FnMut(Option<T>) -> T + Send + Sync + 'static,
    ) -> Self
    where
        T: Send + Sync + 'static,
    {
        let (mut rx, owner, inner) = effect_base();
        let mut first_run = true;
        let value = Arc::new(RwLock::new(None::<T>));

        Executor::spawn({
            let value = Arc::clone(&value);
            let subscriber = inner.to_any_subscriber();

            async move {
                while rx.next().await.is_some() {
                    if first_run
                        || subscriber
                            .with_observer(|| subscriber.update_if_necessary())
                    {
                        first_run = false;
                        subscriber.clear_sources(&subscriber);

                        let old_value =
                            mem::take(&mut *value.write().or_poisoned());
                        let new_value = owner.with_cleanup(|| {
                            subscriber.with_observer(|| fun(old_value))
                        });
                        *value.write().or_poisoned() = Some(new_value);
                    }
                }
            }
        });
        Self {
            inner: StoredValue::new(Some(inner)),
        }
    }
}

impl ToAnySubscriber for Effect {
    fn to_any_subscriber(&self) -> AnySubscriber {
        self.inner
            .try_with_value(|inner| {
                inner.as_ref().map(|inner| inner.to_any_subscriber())
            })
            .flatten()
            .expect("tried to subscribe to effect that has been stopped")
    }
}

#[inline(always)]
#[track_caller]
#[deprecated = "This function is being removed to conform to Rust \
                idioms.Please use `Effect::new()` instead."]
pub fn create_effect<T>(fun: impl FnMut(Option<T>) -> T + 'static) -> Effect
where
    T: 'static,
{
    Effect::new(fun)
}
