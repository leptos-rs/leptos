use crate::{computed::BlockingLock, traits::Trigger};
use core::fmt::Debug;
use guardian::{ArcRwLockReadGuardian, ArcRwLockWriteGuardian};
use std::{
    borrow::Borrow,
    fmt::Display,
    marker::PhantomData,
    ops::{Deref, DerefMut},
    sync::{Arc, RwLock},
};

#[derive(Debug)]
pub struct ReadGuard<T, Inner> {
    ty: PhantomData<T>,
    inner: Inner,
}

impl<T, Inner> ReadGuard<T, Inner> {
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

pub struct Plain<T: 'static> {
    guard: ArcRwLockReadGuardian<T>,
}

impl<T: 'static> Debug for Plain<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Plain").finish()
    }
}

impl<T: 'static> Plain<T> {
    pub(crate) fn try_new(inner: Arc<RwLock<T>>) -> Option<Self> {
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

pub struct AsyncPlain<T: 'static> {
    pub(crate) guard: async_lock::RwLockReadGuardArc<T>,
}

impl<T: 'static> Debug for AsyncPlain<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AsyncPlain").finish()
    }
}

impl<T: 'static> AsyncPlain<T> {
    pub(crate) fn try_new(inner: &Arc<async_lock::RwLock<T>>) -> Option<Self> {
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

#[derive(Debug)]
pub struct Mapped<Inner, U>
where
    Inner: Deref,
{
    inner: Inner,
    map_fn: fn(&Inner::Target) -> &U,
}

impl<T: 'static, U> Mapped<Plain<T>, U> {
    pub(crate) fn try_new(
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
    pub(crate) fn new_with_guard(
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

#[derive(Debug)]
pub struct WriteGuard<'a, S, G>
where
    S: Trigger,
{
    pub(crate) triggerable: Option<&'a S>,
    pub(crate) guard: Option<G>,
}

impl<'a, S, G> WriteGuard<'a, S, G>
where
    S: Trigger,
{
    pub fn new(triggerable: &'a S, guard: G) -> Self {
        Self {
            triggerable: Some(triggerable),
            guard: Some(guard),
        }
    }

    pub fn untrack(&mut self) {
        self.triggerable.take();
    }
}

impl<'a, S, G> Deref for WriteGuard<'a, S, G>
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

impl<'a, S, G> DerefMut for WriteGuard<'a, S, G>
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

pub struct UntrackedWriteGuard<T: 'static>(ArcRwLockWriteGuardian<T>);

impl<T: 'static> UntrackedWriteGuard<T> {
    pub(crate) fn try_new(inner: Arc<RwLock<T>>) -> Option<Self> {
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
impl<'a, S, T> Drop for WriteGuard<'a, S, T>
where
    S: Trigger,
{
    fn drop(&mut self) {
        // first, drop the inner guard
        drop(self.guard.take());

        // then, notify about a change
        if let Some(triggerable) = self.triggerable {
            triggerable.trigger();
        }
    }
}
