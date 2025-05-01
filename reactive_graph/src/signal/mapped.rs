use super::{
    guards::{Mapped, MappedMutArc},
    ArcRwSignal, RwSignal,
};
use crate::{
    owner::{StoredValue, SyncStorage},
    signal::guards::WriteGuard,
    traits::{
        DefinedAt, GetValue, IsDisposed, Notify, ReadUntracked, Track,
        UntrackableGuard, Write,
    },
};
use guardian::ArcRwLockWriteGuardian;
use std::{
    fmt::Debug,
    ops::{Deref, DerefMut},
    panic::Location,
    sync::Arc,
};

/// A derived signal type that wraps an [`ArcRwSignal`] with a mapping function,
///  allowing you to read or write directly to one of its field.
///
/// Tracking the mapped signal tracks changes to *any* part of the signal, and updating the signal notifies
/// and notifies *all* depenendencies of the signal. This is not a mechanism for fine-grained reactive updates
/// to more complex data structures. Instead, it allows you to provide a signal-like API for wrapped types
/// without exposing the original type directly to users.
pub struct ArcMappedSignal<T> {
    #[cfg(any(debug_assertions, leptos_debuginfo))]
    defined_at: &'static Location<'static>,
    #[allow(clippy::type_complexity)]
    try_read_untracked: Arc<
        dyn Fn() -> Option<DoubleDeref<Box<dyn Deref<Target = T>>>>
            + Send
            + Sync,
    >,
    try_write: Arc<
        dyn Fn() -> Option<Box<dyn UntrackableGuard<Target = T>>> + Send + Sync,
    >,
    notify: Arc<dyn Fn() + Send + Sync>,
    track: Arc<dyn Fn() + Send + Sync>,
}

impl<T> Clone for ArcMappedSignal<T> {
    fn clone(&self) -> Self {
        Self {
            #[cfg(any(debug_assertions, leptos_debuginfo))]
            defined_at: self.defined_at,
            try_read_untracked: self.try_read_untracked.clone(),
            try_write: self.try_write.clone(),
            notify: self.notify.clone(),
            track: self.track.clone(),
        }
    }
}

impl<T> ArcMappedSignal<T> {
    /// Wraps a signal with the given mapping functions for shared and exclusive references.
    #[track_caller]
    pub fn new<U>(
        inner: ArcRwSignal<U>,
        map: fn(&U) -> &T,
        map_mut: fn(&mut U) -> &mut T,
    ) -> Self
    where
        T: 'static,
        U: Send + Sync + 'static,
    {
        Self {
            #[cfg(any(debug_assertions, leptos_debuginfo))]
            defined_at: Location::caller(),
            try_read_untracked: {
                let this = inner.clone();
                Arc::new(move || {
                    this.try_read_untracked().map(|guard| DoubleDeref {
                        inner: Box::new(Mapped::new_with_guard(guard, map))
                            as Box<dyn Deref<Target = T>>,
                    })
                })
            },
            try_write: {
                let this = inner.clone();
                Arc::new(move || {
                    let guard = ArcRwLockWriteGuardian::try_take(Arc::clone(
                        &this.value,
                    ))?
                    .ok()?;
                    let mapped = WriteGuard::new(
                        this.clone(),
                        MappedMutArc::new(guard, map, map_mut),
                    );
                    Some(Box::new(mapped))
                })
            },
            notify: {
                let this = inner.clone();
                Arc::new(move || {
                    this.notify();
                })
            },
            track: {
                Arc::new(move || {
                    inner.track();
                })
            },
        }
    }
}

impl<T> Debug for ArcMappedSignal<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut partial = f.debug_struct("ArcMappedSignal");
        #[cfg(any(debug_assertions, leptos_debuginfo))]
        partial.field("defined_at", &self.defined_at);
        partial.finish()
    }
}

impl<T> DefinedAt for ArcMappedSignal<T> {
    fn defined_at(&self) -> Option<&'static Location<'static>> {
        #[cfg(any(debug_assertions, leptos_debuginfo))]
        {
            Some(self.defined_at)
        }
        #[cfg(not(any(debug_assertions, leptos_debuginfo)))]
        {
            None
        }
    }
}

impl<T> Notify for ArcMappedSignal<T> {
    fn notify(&self) {
        (self.notify)()
    }
}

impl<T> Track for ArcMappedSignal<T> {
    fn track(&self) {
        (self.track)()
    }
}

impl<T> ReadUntracked for ArcMappedSignal<T> {
    type Value = DoubleDeref<Box<dyn Deref<Target = T>>>;

    fn try_read_untracked(&self) -> Option<Self::Value> {
        (self.try_read_untracked)()
    }
}

impl<T> IsDisposed for ArcMappedSignal<T> {
    fn is_disposed(&self) -> bool {
        false
    }
}

impl<T> Write for ArcMappedSignal<T>
where
    T: 'static,
{
    type Value = T;

    fn try_write_untracked(
        &self,
    ) -> Option<impl DerefMut<Target = Self::Value>> {
        let mut guard = self.try_write()?;
        guard.untrack();
        Some(guard)
    }

    fn try_write(&self) -> Option<impl UntrackableGuard<Target = Self::Value>> {
        let inner = (self.try_write)()?;
        let inner = DoubleDeref { inner };
        Some(inner)
    }
}

/// A wrapper for a smart pointer that implements [`Deref`] and [`DerefMut`]
/// by dereferencing the type *inside* the smart pointer.
///
/// This is quite obscure and mostly useful for situations in which we want
/// a wrapper for `Box<dyn Deref<Target = T>>` that dereferences to `T` rather
/// than dereferencing to `dyn Deref<Target = T>`.
///
/// This is used internally in [`MappedSignal`] and [`ArcMappedSignal`].
pub struct DoubleDeref<T> {
    inner: T,
}

impl<T> Deref for DoubleDeref<T>
where
    T: Deref,
    T::Target: Deref,
{
    type Target = <T::Target as Deref>::Target;

    fn deref(&self) -> &Self::Target {
        self.inner.deref().deref()
    }
}

impl<T> DerefMut for DoubleDeref<T>
where
    T: DerefMut,
    T::Target: DerefMut,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.inner.deref_mut().deref_mut()
    }
}

impl<T> UntrackableGuard for DoubleDeref<T>
where
    T: UntrackableGuard,
    T::Target: DerefMut,
{
    fn untrack(&mut self) {
        self.inner.untrack();
    }
}

/// A derived signal type that wraps an [`RwSignal`] with a mapping function,
///  allowing you to read or write directly to one of its field.
///
/// Tracking the mapped signal tracks changes to *any* part of the signal, and updating the signal notifies
/// and notifies *all* depenendencies of the signal. This is not a mechanism for fine-grained reactive updates
/// to more complex data structures. Instead, it allows you to provide a signal-like API for wrapped types
/// without exposing the original type directly to users.
pub struct MappedSignal<T, S = SyncStorage> {
    #[cfg(any(debug_assertions, leptos_debuginfo))]
    defined_at: &'static Location<'static>,
    inner: StoredValue<ArcMappedSignal<T>, S>,
}

impl<T> MappedSignal<T> {
    /// Wraps a signal with the given mapping functions for shared and exclusive references.
    #[track_caller]
    pub fn new<U>(
        inner: RwSignal<U>,
        map: fn(&U) -> &T,
        map_mut: fn(&mut U) -> &mut T,
    ) -> Self
    where
        T: Send + Sync + 'static,
        U: Send + Sync + 'static,
    {
        Self {
            #[cfg(any(debug_assertions, leptos_debuginfo))]
            defined_at: Location::caller(),
            inner: {
                let this = ArcRwSignal::from(inner);
                StoredValue::new_with_storage(ArcMappedSignal::new(
                    this, map, map_mut,
                ))
            },
        }
    }
}

impl<T> Copy for MappedSignal<T> {}

impl<T> Clone for MappedSignal<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Debug for MappedSignal<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut partial = f.debug_struct("MappedSignal");
        #[cfg(any(debug_assertions, leptos_debuginfo))]
        partial.field("defined_at", &self.defined_at);
        partial.finish()
    }
}

impl<T> DefinedAt for MappedSignal<T> {
    fn defined_at(&self) -> Option<&'static Location<'static>> {
        #[cfg(any(debug_assertions, leptos_debuginfo))]
        {
            Some(self.defined_at)
        }
        #[cfg(not(any(debug_assertions, leptos_debuginfo)))]
        {
            None
        }
    }
}

impl<T> Notify for MappedSignal<T>
where
    T: 'static,
{
    fn notify(&self) {
        if let Some(inner) = self.inner.try_get_value() {
            inner.notify();
        }
    }
}

impl<T> Track for MappedSignal<T>
where
    T: 'static,
{
    fn track(&self) {
        if let Some(inner) = self.inner.try_get_value() {
            inner.track();
        }
    }
}

impl<T> ReadUntracked for MappedSignal<T>
where
    T: 'static,
{
    type Value = DoubleDeref<Box<dyn Deref<Target = T>>>;

    fn try_read_untracked(&self) -> Option<Self::Value> {
        self.inner
            .try_get_value()
            .and_then(|inner| inner.try_read_untracked())
    }
}

impl<T> Write for MappedSignal<T>
where
    T: 'static,
{
    type Value = T;

    fn try_write_untracked(
        &self,
    ) -> Option<impl DerefMut<Target = Self::Value>> {
        let mut guard = self.try_write()?;
        guard.untrack();
        Some(guard)
    }

    fn try_write(&self) -> Option<impl UntrackableGuard<Target = Self::Value>> {
        let inner = self.inner.try_get_value()?;
        let inner = (inner.try_write)()?;
        let inner = DoubleDeref { inner };
        Some(inner)
    }
}

impl<T> From<ArcMappedSignal<T>> for MappedSignal<T>
where
    T: 'static,
{
    #[track_caller]
    fn from(value: ArcMappedSignal<T>) -> Self {
        MappedSignal {
            #[cfg(any(debug_assertions, leptos_debuginfo))]
            defined_at: Location::caller(),
            inner: StoredValue::new(value),
        }
    }
}

impl<T> IsDisposed for MappedSignal<T> {
    fn is_disposed(&self) -> bool {
        self.inner.is_disposed()
    }
}
