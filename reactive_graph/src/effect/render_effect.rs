use crate::{
    channel::channel,
    effect::inner::EffectInner,
    executor::Executor,
    graph::{AnySubscriber, SourceSet, Subscriber, ToAnySubscriber},
    owner::Owner,
    OrPoisoned,
};
use futures::StreamExt;
use std::{
    fmt::Debug,
    mem,
    sync::{Arc, RwLock, Weak},
};

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
        mut fun: impl FnMut(Option<T>) -> T + 'static,
        initial_value: Option<T>,
    ) -> Self {
        let (observer, mut rx) = channel();
        let value = Arc::new(RwLock::new(None));
        let owner = Owner::new();
        let inner = Arc::new(RwLock::new(EffectInner {
            observer,
            sources: SourceSet::new(),
        }));

        let initial_value = Some(owner.with(|| {
            inner
                .to_any_subscriber()
                .with_observer(|| fun(initial_value))
        }));
        *value.write().or_poisoned() = initial_value;

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
        RenderEffect { value, inner }
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

impl<T> ToAnySubscriber for RenderEffect<T> {
    fn to_any_subscriber(&self) -> AnySubscriber {
        AnySubscriber(
            Arc::as_ptr(&self.inner) as usize,
            Arc::downgrade(&self.inner) as Weak<dyn Subscriber + Send + Sync>,
        )
    }
}
