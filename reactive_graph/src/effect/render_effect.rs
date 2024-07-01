use crate::{
    channel::channel,
    effect::inner::EffectInner,
    graph::{
        AnySubscriber, ReactiveNode, SourceSet, Subscriber, ToAnySubscriber,
    },
    owner::Owner,
};
use any_spawner::Executor;
use futures::StreamExt;
use or_poisoned::OrPoisoned;
use std::{
    fmt::Debug,
    mem,
    sync::{Arc, RwLock, Weak},
};

#[must_use = "A RenderEffect will be canceled when it is dropped. Creating a \
              RenderEffect that is not stored in some other data structure or \
              leaked will drop it immediately, and it will not react to \
              changes in signals it reads."]
pub struct RenderEffect<T>
where
    T: 'static,
{
    value: Arc<RwLock<Option<T>>>,
    inner: Arc<RwLock<EffectInner>>,
}

impl<T> Debug for RenderEffect<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RenderEffect")
            .field("inner", &Arc::as_ptr(&self.inner))
            .finish()
    }
}

impl<T> RenderEffect<T>
where
    T: 'static,
{
    pub fn new(fun: impl FnMut(Option<T>) -> T + 'static) -> Self {
        Self::new_with_value(fun, None)
    }

    pub fn new_with_value(
        fun: impl FnMut(Option<T>) -> T + 'static,
        initial_value: Option<T>,
    ) -> Self {
        fn erased<T>(
            mut fun: Box<dyn FnMut(Option<T>) -> T + 'static>,
            initial_value: Option<T>,
        ) -> RenderEffect<T> {
            let (observer, mut rx) = channel();
            let value = Arc::new(RwLock::new(None::<T>));
            let owner = Owner::new();
            let inner = Arc::new(RwLock::new(EffectInner {
                dirty: false,
                observer,
                sources: SourceSet::new(),
            }));
            crate::log_warning(format_args!(
                "RenderEffect::<{}> owner is {:?} {:?}",
                std::any::type_name::<T>(),
                owner.debug_id(),
                owner.ancestry()
            ));

            let initial_value = cfg!(feature = "effects").then(|| {
                owner.with(|| {
                    inner
                        .to_any_subscriber()
                        .with_observer(|| fun(initial_value))
                })
            });
            *value.write().or_poisoned() = initial_value;

            if cfg!(feature = "effects") {
                Executor::spawn_local({
                    let value = Arc::clone(&value);
                    let subscriber = inner.to_any_subscriber();

                    async move {
                        while rx.next().await.is_some() {
                            if subscriber.with_observer(|| {
                                subscriber.update_if_necessary()
                            }) {
                                subscriber.clear_sources(&subscriber);

                                let old_value = mem::take(
                                    &mut *value.write().or_poisoned(),
                                );
                                let new_value = owner.with_cleanup(|| {
                                    subscriber.with_observer(|| fun(old_value))
                                });
                                *value.write().or_poisoned() = Some(new_value);
                            }
                        }
                    }
                });
            }

            RenderEffect { value, inner }
        }

        erased(Box::new(fun), initial_value)
    }

    pub fn with_value_mut<U>(
        &self,
        fun: impl FnOnce(&mut T) -> U,
    ) -> Option<U> {
        self.value.write().or_poisoned().as_mut().map(fun)
    }

    pub fn take_value(&self) -> Option<T> {
        self.value.write().or_poisoned().take()
    }
}

impl<T> RenderEffect<T>
where
    T: Send + Sync + 'static,
{
    #[doc(hidden)]
    pub fn new_isomorphic(
        mut fun: impl FnMut(Option<T>) -> T + Send + 'static,
    ) -> Self {
        let (mut observer, mut rx) = channel();
        observer.notify();

        let value = Arc::new(RwLock::new(None::<T>));
        let owner = Owner::new();
        let inner = Arc::new(RwLock::new(EffectInner {
            dirty: false,
            observer,
            sources: SourceSet::new(),
        }));
        let mut first_run = true;

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
        RenderEffect { value, inner }
    }
}

impl<T> ToAnySubscriber for RenderEffect<T> {
    fn to_any_subscriber(&self) -> AnySubscriber {
        AnySubscriber(
            Arc::as_ptr(&self.inner) as usize,
            Arc::downgrade(&self.inner) as Weak<dyn Subscriber + Send + Sync>,
        )
    }
}
