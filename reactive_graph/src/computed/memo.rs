use super::{inner::MemoInner, ArcMemo};
use crate::{
    owner::StoredValue,
    signal::guards::{Mapped, Plain, ReadGuard},
    traits::{DefinedAt, Dispose, ReadUntracked, Track},
    unwrap_signal,
};
use std::{fmt::Debug, hash::Hash, panic::Location};

pub struct Memo<T: Send + Sync + 'static> {
    #[cfg(debug_assertions)]
    defined_at: &'static Location<'static>,
    inner: StoredValue<ArcMemo<T>>,
}

impl<T: Send + Sync + 'static> Dispose for Memo<T> {
    fn dispose(self) {
        self.inner.dispose()
    }
}

impl<T: Send + Sync + 'static> From<ArcMemo<T>> for Memo<T> {
    #[track_caller]
    fn from(value: ArcMemo<T>) -> Self {
        Self {
            #[cfg(debug_assertions)]
            defined_at: Location::caller(),
            inner: StoredValue::new(value),
        }
    }
}

impl<T: Send + Sync + 'static> Memo<T> {
    #[track_caller]
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(level = "debug", skip_all,)
    )]
    pub fn new(fun: impl Fn(Option<&T>) -> T + Send + Sync + 'static) -> Self
    where
        T: PartialEq,
    {
        Self {
            #[cfg(debug_assertions)]
            defined_at: Location::caller(),
            inner: StoredValue::new(ArcMemo::new(fun)),
        }
    }
}

impl<T: Send + Sync + 'static> Copy for Memo<T> {}

impl<T: Send + Sync + 'static> Clone for Memo<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: Send + Sync + 'static> Debug for Memo<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Memo")
            .field("type", &std::any::type_name::<T>())
            .field("store", &self.inner)
            .finish()
    }
}

impl<T: Send + Sync + 'static> PartialEq for Memo<T> {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

impl<T: Send + Sync + 'static> Eq for Memo<T> {}

impl<T: Send + Sync + 'static> Hash for Memo<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.inner.hash(state);
    }
}

impl<T: Send + Sync + 'static> DefinedAt for Memo<T> {
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

impl<T: Send + Sync + 'static> Track for Memo<T> {
    fn track(&self) {
        if let Some(inner) = self.inner.get() {
            inner.track();
        }
    }
}

impl<T: Send + Sync + 'static> ReadUntracked for Memo<T> {
    type Value = ReadGuard<T, Mapped<Plain<MemoInner<T>>, T>>;

    fn try_read_untracked(&self) -> Option<Self::Value> {
        self.inner.get().map(|inner| inner.read_untracked())
    }
}

impl<T: Send + Sync + 'static> From<Memo<T>> for ArcMemo<T> {
    #[track_caller]
    fn from(value: Memo<T>) -> Self {
        value.inner.get().unwrap_or_else(unwrap_signal!(value))
    }
}
