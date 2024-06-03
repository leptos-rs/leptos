use super::inner::MemoInner;
use crate::{
    graph::{
        AnySource, AnySubscriber, ReactiveNode, Source, Subscriber,
        ToAnySource, ToAnySubscriber,
    },
    signal::guards::{Mapped, Plain, ReadGuard},
    traits::{DefinedAt, ReadUntracked},
};
use core::fmt::Debug;
use or_poisoned::OrPoisoned;
use std::{
    hash::Hash,
    panic::Location,
    sync::{Arc, RwLock, Weak},
};

pub struct ArcMemo<T> {
    #[cfg(debug_assertions)]
    defined_at: &'static Location<'static>,
    inner: Arc<RwLock<MemoInner<T>>>,
}

impl<T: Send + Sync + 'static> ArcMemo<T> {
    #[track_caller]
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(level = "trace", skip_all,)
    )]
    pub fn new(fun: impl Fn(Option<&T>) -> T + Send + Sync + 'static) -> Self
    where
        T: PartialEq,
    {
        Self::new_with_compare(fun, |lhs, rhs| lhs.as_ref() != rhs.as_ref())
    }

    #[track_caller]
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(level = "trace", skip_all,)
    )]
    pub fn new_with_compare(
        fun: impl Fn(Option<&T>) -> T + Send + Sync + 'static,
        changed: fn(Option<&T>, Option<&T>) -> bool,
    ) -> Self
    where
        T: PartialEq,
    {
        Self::new_owning(move |prev: Option<T>| {
            let new_value = fun(prev.as_ref());
            let changed = changed(prev.as_ref(), Some(&new_value));
            (new_value, changed)
        })
    }

    #[track_caller]
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(level = "trace", skip_all,)
    )]
    pub fn new_owning(
        fun: impl Fn(Option<T>) -> (T, bool) + Send + Sync + 'static,
    ) -> Self
    where
        T: PartialEq,
    {
        let inner = Arc::new_cyclic(|weak| {
            let subscriber = AnySubscriber(
                weak.as_ptr() as usize,
                Weak::clone(weak) as Weak<dyn Subscriber + Send + Sync>,
            );

            RwLock::new(MemoInner::new(Arc::new(fun), subscriber))
        });
        Self {
            #[cfg(debug_assertions)]
            defined_at: Location::caller(),
            inner,
        }
    }
}

impl<T> Clone for ArcMemo<T> {
    fn clone(&self) -> Self {
        Self {
            #[cfg(debug_assertions)]
            defined_at: self.defined_at,
            inner: Arc::clone(&self.inner),
        }
    }
}

impl<T> DefinedAt for ArcMemo<T> {
    #[inline(always)]
    fn defined_at(&self) -> Option<&'static Location<'static>> {
        #[cfg(debug_assertions)]
        {
            Some(self.defined_at)
        }
        #[cfg(not(debug_assertions))]
        {
            None
        }
    }
}

impl<T> Debug for ArcMemo<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ArcMemo")
            .field("type", &std::any::type_name::<T>())
            .field("data", &Arc::as_ptr(&self.inner))
            .finish()
    }
}

impl<T> PartialEq for ArcMemo<T> {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.inner, &other.inner)
    }
}

impl<T> Eq for ArcMemo<T> {}

impl<T> Hash for ArcMemo<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::ptr::hash(&Arc::as_ptr(&self.inner), state);
    }
}

impl<T: 'static> ReactiveNode for ArcMemo<T> {
    fn mark_dirty(&self) {
        self.inner.mark_dirty();
    }

    fn mark_check(&self) {
        self.inner.mark_check();
    }

    fn mark_subscribers_check(&self) {
        self.inner.mark_subscribers_check();
    }

    fn update_if_necessary(&self) -> bool {
        self.inner.update_if_necessary()
    }
}

impl<T: Send + Sync + 'static> ToAnySource for ArcMemo<T> {
    fn to_any_source(&self) -> AnySource {
        AnySource(
            Arc::as_ptr(&self.inner) as usize,
            Arc::downgrade(&self.inner) as Weak<dyn Source + Send + Sync>,
            #[cfg(debug_assertions)]
            self.defined_at,
        )
    }
}

impl<T: 'static> Source for ArcMemo<T> {
    fn add_subscriber(&self, subscriber: AnySubscriber) {
        self.inner.add_subscriber(subscriber);
    }

    fn remove_subscriber(&self, subscriber: &AnySubscriber) {
        self.inner.remove_subscriber(subscriber);
    }

    fn clear_subscribers(&self) {
        self.inner.clear_subscribers();
    }
}

impl<T: Send + Sync + 'static> ToAnySubscriber for ArcMemo<T> {
    fn to_any_subscriber(&self) -> AnySubscriber {
        AnySubscriber(
            Arc::as_ptr(&self.inner) as usize,
            Arc::downgrade(&self.inner) as Weak<dyn Subscriber + Send + Sync>,
        )
    }
}

impl<T: 'static> Subscriber for ArcMemo<T> {
    fn add_source(&self, source: AnySource) {
        self.inner.write().or_poisoned().sources.insert(source);
    }

    fn clear_sources(&self, subscriber: &AnySubscriber) {
        self.inner
            .write()
            .or_poisoned()
            .sources
            .clear_sources(subscriber);
    }
}

impl<T: 'static> ReadUntracked for ArcMemo<T> {
    type Value = ReadGuard<T, Mapped<Plain<MemoInner<T>>, T>>;

    fn try_read_untracked(&self) -> Option<Self::Value> {
        self.update_if_necessary();

        Mapped::try_new(Arc::clone(&self.inner), |t| {
            // safe to unwrap here because update_if_necessary
            // guarantees the value is Some
            t.value.as_ref().unwrap()
        })
        .map(ReadGuard::new)
    }
}
