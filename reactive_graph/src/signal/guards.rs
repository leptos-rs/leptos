use crate::traits::Trigger;
use core::fmt::Debug;
use guardian::ArcRwLockReadGuardian;
use std::{
    fmt::Display,
    ops::{Deref, DerefMut},
    sync::{Arc, RwLock, RwLockWriteGuard},
};

pub struct SignalReadGuard<T: 'static> {
    guard: ArcRwLockReadGuardian<T>,
}

impl<T: 'static> Debug for SignalReadGuard<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SignalReadGuard").finish()
    }
}

impl<T: 'static> SignalReadGuard<T> {
    pub fn try_new(inner: Arc<RwLock<T>>) -> Option<Self> {
        ArcRwLockReadGuardian::take(inner)
            .ok()
            .map(|guard| SignalReadGuard { guard })
    }
}

impl<T> Deref for SignalReadGuard<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.guard.deref()
    }
}

impl<T: PartialEq> PartialEq for SignalReadGuard<T> {
    fn eq(&self, other: &Self) -> bool {
        **self == **other
    }
}

impl<T: PartialEq> PartialEq<T> for SignalReadGuard<T> {
    fn eq(&self, other: &T) -> bool {
        **self == *other
    }
}

impl<T: Display> Display for SignalReadGuard<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&**self, f)
    }
}

pub struct MappedSignalReadGuard<T: 'static, U> {
    guard: ArcRwLockReadGuardian<T>,
    map_fn: fn(&T) -> &U,
}

impl<T: 'static, U> Debug for MappedSignalReadGuard<T, U> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MappedSignalReadGuard").finish()
    }
}

impl<T: 'static, U> MappedSignalReadGuard<T, U> {
    pub fn try_new(
        inner: Arc<RwLock<T>>,
        map_fn: fn(&T) -> &U,
    ) -> Option<Self> {
        ArcRwLockReadGuardian::take(inner)
            .ok()
            .map(|guard| MappedSignalReadGuard { guard, map_fn })
    }
}

impl<T, U> Deref for MappedSignalReadGuard<T, U> {
    type Target = U;

    fn deref(&self) -> &Self::Target {
        (self.map_fn)(self.guard.deref())
    }
}

impl<T, U: PartialEq> PartialEq for MappedSignalReadGuard<T, U> {
    fn eq(&self, other: &Self) -> bool {
        **self == **other
    }
}

impl<T, U: PartialEq> PartialEq<U> for MappedSignalReadGuard<T, U> {
    fn eq(&self, other: &U) -> bool {
        **self == *other
    }
}

impl<T, U: Display> Display for MappedSignalReadGuard<T, U> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&**self, f)
    }
}

#[derive(Debug)]
pub struct SignalWriteGuard<'a, S, T>
where
    S: Trigger,
{
    triggerable: &'a S,
    guard: Option<RwLockWriteGuard<'a, T>>,
}

impl<'a, S, T> SignalWriteGuard<'a, S, T>
where
    S: Trigger,
{
    pub fn new(triggerable: &'a S, guard: RwLockWriteGuard<'a, T>) -> Self {
        Self {
            guard: Some(guard),
            triggerable,
        }
    }
}

impl<'a, S, T> Deref for SignalWriteGuard<'a, S, T>
where
    S: Trigger,
{
    type Target = T;

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

impl<'a, S, T> DerefMut for SignalWriteGuard<'a, S, T>
where
    S: Trigger,
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

#[derive(Debug)]
pub struct SignalUntrackedWriteGuard<'a, T>(RwLockWriteGuard<'a, T>);

impl<'a, T> From<RwLockWriteGuard<'a, T>> for SignalUntrackedWriteGuard<'a, T> {
    fn from(value: RwLockWriteGuard<'a, T>) -> Self {
        Self(value)
    }
}

impl<'a, T> Deref for SignalUntrackedWriteGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}

impl<'a, T> DerefMut for SignalUntrackedWriteGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.deref_mut()
    }
}

// Dropping the write guard will notify dependencies.
impl<'a, S, T> Drop for SignalWriteGuard<'a, S, T>
where
    S: Trigger,
{
    fn drop(&mut self) {
        // first, drop the inner guard
        drop(self.guard.take());

        // then, notify about a change
        self.triggerable.trigger();
    }
}
