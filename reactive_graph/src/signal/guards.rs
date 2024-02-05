use crate::traits::Trigger;
use std::{
    ops::{Deref, DerefMut},
    sync::{RwLockReadGuard, RwLockWriteGuard},
};

#[derive(Debug)]
pub struct SignalReadGuard<'a, T, U> {
    guard: RwLockReadGuard<'a, T>,
    map_fn: fn(&T) -> &U,
}

impl<'a, T> SignalReadGuard<'a, T, T> {
    pub fn new(guard: RwLockReadGuard<'a, T>) -> Self {
        SignalReadGuard {
            guard,
            map_fn: |t| t,
        }
    }
}

impl<'a, T, U> SignalReadGuard<'a, T, U> {
    pub fn new_with_map_fn(
        guard: RwLockReadGuard<'a, T>,
        map_fn: fn(&T) -> &U,
    ) -> Self {
        SignalReadGuard { guard, map_fn }
    }
}

impl<'a, T> From<RwLockReadGuard<'a, T>> for SignalReadGuard<'a, T, T> {
    fn from(guard: RwLockReadGuard<'a, T>) -> Self {
        SignalReadGuard::new(guard)
    }
}

impl<'a, T, U> Deref for SignalReadGuard<'a, T, U> {
    type Target = U;

    fn deref(&self) -> &Self::Target {
        (self.map_fn)(self.guard.deref())
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
