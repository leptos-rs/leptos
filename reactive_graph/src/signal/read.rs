use super::{
    arc_signal,
    guards::{Plain, ReadGuard},
    subscriber_traits::AsSubscriberSet,
    ArcReadSignal,
};
use crate::{
    graph::SubscriberSet,
    owner::StoredValue,
    traits::{DefinedAt, Dispose, IsDisposed, ReadUntracked, Set},
    unwrap_signal,
};
use any_spawner::Executor;
use core::fmt::Debug;
use futures::{Stream, StreamExt};
use std::{
    hash::Hash,
    panic::Location,
    sync::{Arc, RwLock},
};

pub struct ReadSignal<T: 'static> {
    #[cfg(debug_assertions)]
    pub(crate) defined_at: &'static Location<'static>,
    pub(crate) inner: StoredValue<ArcReadSignal<T>>,
}

impl<T: 'static> Dispose for ReadSignal<T> {
    fn dispose(self) {
        self.inner.dispose()
    }
}

impl<T> Copy for ReadSignal<T> {}

impl<T> Clone for ReadSignal<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Debug for ReadSignal<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ReadSignal")
            .field("type", &std::any::type_name::<T>())
            .field("store", &self.inner)
            .finish()
    }
}

impl<T> PartialEq for ReadSignal<T> {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

impl<T> Eq for ReadSignal<T> {}

impl<T> Hash for ReadSignal<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.inner.hash(state);
    }
}

impl<T> DefinedAt for ReadSignal<T> {
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

impl<T: 'static> IsDisposed for ReadSignal<T> {
    fn is_disposed(&self) -> bool {
        self.inner.exists()
    }
}

impl<T> AsSubscriberSet for ReadSignal<T> {
    type Output = Arc<RwLock<SubscriberSet>>;

    fn as_subscriber_set(&self) -> Option<Self::Output> {
        self.inner
            .try_with_value(|inner| inner.as_subscriber_set())
            .flatten()
    }
}

impl<T: 'static> ReadUntracked for ReadSignal<T> {
    type Value = ReadGuard<T, Plain<T>>;

    fn try_read_untracked(&self) -> Option<Self::Value> {
        self.inner
            .try_get_value()
            .map(|inner| inner.read_untracked())
    }
}

impl<T: Send + Sync + 'static> From<ArcReadSignal<T>> for ReadSignal<T> {
    #[track_caller]
    fn from(value: ArcReadSignal<T>) -> Self {
        ReadSignal {
            #[cfg(debug_assertions)]
            defined_at: Location::caller(),
            inner: StoredValue::new(value),
        }
    }
}

impl<T: Send + Sync + 'static> From<ReadSignal<T>> for ArcReadSignal<T> {
    #[track_caller]
    fn from(value: ReadSignal<T>) -> Self {
        value.inner.get().unwrap_or_else(unwrap_signal!(value))
    }
}

impl<T: Send + Sync + 'static> ArcReadSignal<T> {
    pub fn from_stream(
        stream: impl Stream<Item = T> + Send + 'static,
    ) -> ArcReadSignal<Option<T>> {
        let (read, write) = arc_signal(None);
        let mut stream = Box::pin(stream);
        Executor::spawn(async move {
            while let Some(value) = stream.next().await {
                write.set(Some(value));
            }
        });
        read
    }
}

impl<T: 'static> ArcReadSignal<T> {
    pub fn from_stream_unsync(
        stream: impl Stream<Item = T> + 'static,
    ) -> ArcReadSignal<Option<T>> {
        let (read, write) = arc_signal(None);
        let mut stream = Box::pin(stream);
        Executor::spawn_local(async move {
            while let Some(value) = stream.next().await {
                write.set(Some(value));
            }
        });
        read
    }
}

impl<T: Send + Sync + 'static> ReadSignal<T> {
    pub fn from_stream(
        stream: impl Stream<Item = T> + Send + 'static,
    ) -> ArcReadSignal<Option<T>> {
        let (read, write) = arc_signal(None);
        let mut stream = Box::pin(stream);
        Executor::spawn(async move {
            while let Some(value) = stream.next().await {
                write.set(Some(value));
            }
        });
        read
    }
}

impl<T: 'static> ReadSignal<T> {
    pub fn from_stream_unsync(
        stream: impl Stream<Item = T> + 'static,
    ) -> ArcReadSignal<Option<T>> {
        let (read, write) = arc_signal(None);
        let mut stream = Box::pin(stream);
        Executor::spawn_local(async move {
            while let Some(value) = stream.next().await {
                write.set(Some(value));
            }
        });
        read
    }
}
