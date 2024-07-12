//! Guards that integrate with the reactive system, wrapping references to the values of signals.

use crate::{
    computed::BlockingLock,
    traits::{Trigger, UntrackableGuard},
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
    pub fn try_new(inner: Arc<RwLock<T>>) -> Option<Self> {
        ArcRwLockReadGuardian::take(inner)
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
    S: Trigger,
{
    pub(crate) triggerable: Option<S>,
    pub(crate) guard: Option<G>,
}

impl<S, G> WriteGuard<S, G>
where
    S: Trigger,
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
    S: Trigger,
    G: DerefMut,
{
    /// Removes the triggerable type, so that it is no longer notifies when dropped.
    fn untrack(&mut self) {
        self.triggerable.take();
    }
}

impl<S, G> Deref for WriteGuard<S, G>
where
    S: Trigger,
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
    S: Trigger,
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
    pub fn try_new(inner: Arc<RwLock<T>>) -> Option<Self> {
        ArcRwLockWriteGuardian::take(inner)
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
    S: Trigger,
{
    fn drop(&mut self) {
        // first, drop the inner guard
        drop(self.guard.take());

        // then, notify about a change
        if let Some(triggerable) = self.triggerable.as_ref() {
            triggerable.trigger();
        }
    }
}

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

pub struct MappedArc<Inner, U>
where
    Inner: Deref,
{
    inner: Inner,
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

pub struct MappedMutArc<Inner, U>
where
    Inner: Deref,
{
    inner: Inner,
    map_fn: Arc<dyn Fn(&Inner::Target) -> &U>,
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
