//! Guards that integrate with the reactive system, wrapping references to the values of signals.

use crate::{
    computed::BlockingLock,
    traits::{Notify, UntrackableGuard},
};
use core::fmt::Debug;
use guardian::{ArcRwLockReadGuardian, ArcRwLockWriteGuardian};
use std::{
    borrow::Borrow,
    fmt::Display,
    marker::PhantomData,
    ops::{Deref, DerefMut},
    sync::{Arc, RwLock},
};

/// A wrapper type for any kind of guard returned by [`Read`](crate::traits::Read).
///
/// If `Inner` implements `Deref`, so does `ReadGuard<_, Inner>`.
#[derive(Debug)]
pub struct ReadGuard<T, Inner> {
    ty: PhantomData<T>,
    inner: Inner,
}

impl<T, Inner> ReadGuard<T, Inner> {
    /// Creates a new wrapper around another guard type.
    pub fn new(inner: Inner) -> Self {
        Self {
            inner,
            ty: PhantomData,
        }
    }

    /// Returns the inner guard type.
    pub fn into_inner(self) -> Inner {
        self.inner
    }
}

impl<T, Inner> Clone for ReadGuard<T, Inner>
where
    Inner: Clone,
{
    fn clone(&self) -> Self {
        Self {
            ty: self.ty,
            inner: self.inner.clone(),
        }
    }
}

impl<T, Inner> Deref for ReadGuard<T, Inner>
where
    Inner: Deref<Target = T>,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.inner.deref()
    }
}

impl<T, Inner> Borrow<T> for ReadGuard<T, Inner>
where
    Inner: Deref<Target = T>,
{
    fn borrow(&self) -> &T {
        self.deref()
    }
}

impl<T, Inner> PartialEq<T> for ReadGuard<T, Inner>
where
    Inner: Deref<Target = T>,
    T: PartialEq,
{
    fn eq(&self, other: &Inner::Target) -> bool {
        self.deref() == other
    }
}

impl<T, Inner> Display for ReadGuard<T, Inner>
where
    Inner: Deref<Target = T>,
    T: Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&**self, f)
    }
}

/// A guard that provides access to a signal's inner value.
pub struct Plain<T: 'static> {
    guard: ArcRwLockReadGuardian<T>,
}

impl<T: 'static> Debug for Plain<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Plain").finish()
    }
}

impl<T: 'static> Plain<T> {
    /// Takes a reference-counted read guard on the given lock.
    pub fn try_new(inner: Arc<RwLock<T>>) -> Option<Self> {
        ArcRwLockReadGuardian::try_take(inner)?
            .ok()
            .map(|guard| Plain { guard })
    }
}

impl<T> Deref for Plain<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.guard.deref()
    }
}

impl<T: PartialEq> PartialEq for Plain<T> {
    fn eq(&self, other: &Self) -> bool {
        **self == **other
    }
}

impl<T: PartialEq> PartialEq<T> for Plain<T> {
    fn eq(&self, other: &T) -> bool {
        **self == *other
    }
}

impl<T: Display> Display for Plain<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&**self, f)
    }
}

/// A guard that provides access to an async signal's value.
pub struct AsyncPlain<T: 'static> {
    pub(crate) guard: async_lock::RwLockReadGuardArc<T>,
}

impl<T: 'static> Debug for AsyncPlain<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AsyncPlain").finish()
    }
}

impl<T: 'static> AsyncPlain<T> {
    /// Takes a reference-counted async read guard on the given lock.
    pub fn try_new(inner: &Arc<async_lock::RwLock<T>>) -> Option<Self> {
        Some(Self {
            guard: inner.blocking_read_arc(),
        })
    }
}

impl<T> Deref for AsyncPlain<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.guard.deref()
    }
}

impl<T: PartialEq> PartialEq for AsyncPlain<T> {
    fn eq(&self, other: &Self) -> bool {
        **self == **other
    }
}

impl<T: PartialEq> PartialEq<T> for AsyncPlain<T> {
    fn eq(&self, other: &T) -> bool {
        **self == *other
    }
}

impl<T: Display> Display for AsyncPlain<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&**self, f)
    }
}

/// A guard that maps over another guard.
#[derive(Debug)]
pub struct Mapped<Inner, U>
where
    Inner: Deref,
{
    inner: Inner,
    map_fn: fn(&Inner::Target) -> &U,
}

impl<T: 'static, U> Mapped<Plain<T>, U> {
    /// Creates a mapped read guard from the inner lock.
    pub fn try_new(
        inner: Arc<RwLock<T>>,
        map_fn: fn(&T) -> &U,
    ) -> Option<Self> {
        let inner = Plain::try_new(inner)?;
        Some(Self { inner, map_fn })
    }
}

impl<Inner, U> Mapped<Inner, U>
where
    Inner: Deref,
{
    /// Creates a mapped read guard from the inner guard.
    pub fn new_with_guard(
        inner: Inner,
        map_fn: fn(&Inner::Target) -> &U,
    ) -> Self {
        Self { inner, map_fn }
    }
}

impl<Inner, U> Deref for Mapped<Inner, U>
where
    Inner: Deref,
{
    type Target = U;

    fn deref(&self) -> &Self::Target {
        (self.map_fn)(self.inner.deref())
    }
}

impl<Inner, U: PartialEq> PartialEq for Mapped<Inner, U>
where
    Inner: Deref,
{
    fn eq(&self, other: &Self) -> bool {
        **self == **other
    }
}

impl<Inner, U: PartialEq> PartialEq<U> for Mapped<Inner, U>
where
    Inner: Deref,
{
    fn eq(&self, other: &U) -> bool {
        **self == *other
    }
}

impl<Inner, U: Display> Display for Mapped<Inner, U>
where
    Inner: Deref,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&**self, f)
    }
}

/// A guard that provides mutable access to a signal's value, triggering some reactive change
/// when it is dropped.
#[derive(Debug)]
pub struct WriteGuard<S, G>
where
    S: Notify,
{
    pub(crate) triggerable: Option<S>,
    pub(crate) guard: Option<G>,
}

impl<S, G> WriteGuard<S, G>
where
    S: Notify,
{
    /// Creates a new guard from the inner mutable guard type, and the signal that should be
    /// triggered on drop.
    pub fn new(triggerable: S, guard: G) -> Self {
        Self {
            triggerable: Some(triggerable),
            guard: Some(guard),
        }
    }
}

impl<S, G> UntrackableGuard for WriteGuard<S, G>
where
    S: Notify,
    G: DerefMut,
{
    /// Removes the triggerable type, so that it is no longer notifies when dropped.
    fn untrack(&mut self) {
        self.triggerable.take();
    }
}

impl<S, G> Deref for WriteGuard<S, G>
where
    S: Notify,
    G: Deref,
{
    type Target = G::Target;

    fn deref(&self) -> &Self::Target {
        self.guard
            .as_ref()
            .expect(
                "the guard should always be in place until the Drop \
                 implementation",
            )
            .deref()
    }
}

impl<S, G> DerefMut for WriteGuard<S, G>
where
    S: Notify,
    G: DerefMut,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.guard
            .as_mut()
            .expect(
                "the guard should always be in place until the Drop \
                 implementation",
            )
            .deref_mut()
    }
}

/// A guard that provides mutable access to a signal's inner value, but does not notify of any
/// changes.
pub struct UntrackedWriteGuard<T: 'static>(ArcRwLockWriteGuardian<T>);

impl<T: 'static> UntrackedWriteGuard<T> {
    /// Creates a write guard from the given lock.
    pub fn try_new(inner: Arc<RwLock<T>>) -> Option<Self> {
        ArcRwLockWriteGuardian::try_take(inner)?
            .ok()
            .map(UntrackedWriteGuard)
    }
}

impl<T> Deref for UntrackedWriteGuard<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}

impl<T> DerefMut for UntrackedWriteGuard<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.deref_mut()
    }
}

// Dropping the write guard will notify dependencies.
impl<S, T> Drop for WriteGuard<S, T>
where
    S: Notify,
{
    fn drop(&mut self) {
        // first, drop the inner guard
        drop(self.guard.take());

        // then, notify about a change
        if let Some(triggerable) = self.triggerable.as_ref() {
            triggerable.notify();
        }
    }
}

/// A mutable guard that maps over an inner mutable guard.
#[derive(Debug)]
pub struct MappedMut<Inner, U>
where
    Inner: Deref,
{
    inner: Inner,
    map_fn: fn(&Inner::Target) -> &U,
    map_fn_mut: fn(&mut Inner::Target) -> &mut U,
}

impl<Inner, U> UntrackableGuard for MappedMut<Inner, U>
where
    Inner: UntrackableGuard,
{
    fn untrack(&mut self) {
        self.inner.untrack();
    }
}

impl<Inner, U> MappedMut<Inner, U>
where
    Inner: DerefMut,
{
    /// Creates a new writable guard from the inner guard.
    pub fn new(
        inner: Inner,
        map_fn: fn(&Inner::Target) -> &U,
        map_fn_mut: fn(&mut Inner::Target) -> &mut U,
    ) -> Self {
        Self {
            inner,
            map_fn,
            map_fn_mut,
        }
    }
}

impl<Inner, U> Deref for MappedMut<Inner, U>
where
    Inner: Deref,
{
    type Target = U;

    fn deref(&self) -> &Self::Target {
        (self.map_fn)(self.inner.deref())
    }
}

impl<Inner, U> DerefMut for MappedMut<Inner, U>
where
    Inner: DerefMut,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        (self.map_fn_mut)(self.inner.deref_mut())
    }
}

impl<Inner, U: PartialEq> PartialEq for MappedMut<Inner, U>
where
    Inner: Deref,
{
    fn eq(&self, other: &Self) -> bool {
        **self == **other
    }
}

impl<Inner, U: Display> Display for MappedMut<Inner, U>
where
    Inner: Deref,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&**self, f)
    }
}

/// A mapped read guard in which the mapping function is a closure. If the mapping function is a
/// function pointer, use [`Mapped`].
pub struct MappedArc<Inner, U>
where
    Inner: Deref,
{
    inner: Inner,
    #[allow(clippy::type_complexity)]
    map_fn: Arc<dyn Fn(&Inner::Target) -> &U>,
}

impl<Inner, U> Clone for MappedArc<Inner, U>
where
    Inner: Clone + Deref,
{
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            map_fn: self.map_fn.clone(),
        }
    }
}

impl<Inner, U> Debug for MappedArc<Inner, U>
where
    Inner: Debug + Deref,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MappedArc")
            .field("inner", &self.inner)
            .finish_non_exhaustive()
    }
}

impl<Inner, U> MappedArc<Inner, U>
where
    Inner: Deref,
{
    /// Creates a new mapped guard from the inner guard and the map function.
    pub fn new(
        inner: Inner,
        map_fn: impl Fn(&Inner::Target) -> &U + 'static,
    ) -> Self {
        Self {
            inner,
            map_fn: Arc::new(map_fn),
        }
    }
}

impl<Inner, U> Deref for MappedArc<Inner, U>
where
    Inner: Deref,
{
    type Target = U;

    fn deref(&self) -> &Self::Target {
        (self.map_fn)(self.inner.deref())
    }
}

impl<Inner, U: PartialEq> PartialEq for MappedArc<Inner, U>
where
    Inner: Deref,
{
    fn eq(&self, other: &Self) -> bool {
        **self == **other
    }
}

impl<Inner, U: Display> Display for MappedArc<Inner, U>
where
    Inner: Deref,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&**self, f)
    }
}

/// A mapped write guard in which the mapping function is a closure. If the mapping function is a
/// function pointer, use [`MappedMut`].
pub struct MappedMutArc<Inner, U>
where
    Inner: Deref,
{
    inner: Inner,
    #[allow(clippy::type_complexity)]
    map_fn: Arc<dyn Fn(&Inner::Target) -> &U>,
    #[allow(clippy::type_complexity)]
    map_fn_mut: Arc<dyn Fn(&mut Inner::Target) -> &mut U>,
}

impl<Inner, U> Clone for MappedMutArc<Inner, U>
where
    Inner: Clone + Deref,
{
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            map_fn: self.map_fn.clone(),
            map_fn_mut: self.map_fn_mut.clone(),
        }
    }
}

impl<Inner, U> Debug for MappedMutArc<Inner, U>
where
    Inner: Debug + Deref,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MappedMutArc")
            .field("inner", &self.inner)
            .finish_non_exhaustive()
    }
}

impl<Inner, U> UntrackableGuard for MappedMutArc<Inner, U>
where
    Inner: UntrackableGuard,
{
    fn untrack(&mut self) {
        self.inner.untrack();
    }
}

impl<Inner, U> MappedMutArc<Inner, U>
where
    Inner: Deref,
{
    /// Creates the new mapped mutable guard from the inner guard and mapping functions.
    pub fn new(
        inner: Inner,
        map_fn: impl Fn(&Inner::Target) -> &U + 'static,
        map_fn_mut: impl Fn(&mut Inner::Target) -> &mut U + 'static,
    ) -> Self {
        Self {
            inner,
            map_fn: Arc::new(map_fn),
            map_fn_mut: Arc::new(map_fn_mut),
        }
    }
}

impl<Inner, U> Deref for MappedMutArc<Inner, U>
where
    Inner: Deref,
{
    type Target = U;

    fn deref(&self) -> &Self::Target {
        (self.map_fn)(self.inner.deref())
    }
}

impl<Inner, U> DerefMut for MappedMutArc<Inner, U>
where
    Inner: DerefMut,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        (self.map_fn_mut)(self.inner.deref_mut())
    }
}

impl<Inner, U: PartialEq> PartialEq for MappedMutArc<Inner, U>
where
    Inner: Deref,
{
    fn eq(&self, other: &Self) -> bool {
        **self == **other
    }
}

impl<Inner, U: Display> Display for MappedMutArc<Inner, U>
where
    Inner: Deref,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&**self, f)
    }
}

/// A wrapper that implements [`Deref`] and [`Borrow`] for itself.
pub struct Derefable<T>(pub T);

impl<T> Clone for Derefable<T>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        Derefable(self.0.clone())
    }
}

impl<T> std::ops::Deref for Derefable<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> Borrow<T> for Derefable<T> {
    fn borrow(&self) -> &T {
        self.deref()
    }
}

impl<T> PartialEq<T> for Derefable<T>
where
    T: PartialEq,
{
    fn eq(&self, other: &T) -> bool {
        self.deref() == other
    }
}

impl<T> Display for Derefable<T>
where
    T: Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&**self, f)
    }
}
