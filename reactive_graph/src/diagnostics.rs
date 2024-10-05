//! By default, attempting to [`Track`](crate::traits::Track) a signal when you are not in a
//! reactive tracking context will cause a warning when you are in debug mode.
//!
//! In some cases, this warning is a false positive. For example, inside an event listener in a
//! user interface, you never want to read from a signal reactively; the event listener should run
//! when the event fires, not when a signal read in the event listener changes.
//!
//! This module provides utilities to suppress those warnings by entering a
//! [`SpecialNonReactiveZone`].

/// Marks an execution block that is known not to be reactive, and suppresses warnings.
#[derive(Debug)]
pub struct SpecialNonReactiveZone;

/// Exits the "special non-reactive zone" when dropped.
#[derive(Debug)]
pub struct SpecialNonReactiveZoneGuard;

use pin_project_lite::pin_project;
use std::{
    cell::Cell,
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

thread_local! {
    static IS_SPECIAL_ZONE: Cell<bool> = const { Cell::new(false) };
}

impl SpecialNonReactiveZone {
    /// Suppresses warnings about non-reactive accesses until the guard is dropped.
    pub fn enter() -> SpecialNonReactiveZoneGuard {
        IS_SPECIAL_ZONE.set(true);
        SpecialNonReactiveZoneGuard
    }

    #[cfg(all(debug_assertions, feature = "effects"))]
    #[inline(always)]
    pub(crate) fn is_inside() -> bool {
        if cfg!(debug_assertions) {
            IS_SPECIAL_ZONE.get()
        } else {
            false
        }
    }
}

impl Drop for SpecialNonReactiveZoneGuard {
    fn drop(&mut self) {
        IS_SPECIAL_ZONE.set(false);
    }
}

pin_project! {
    #[doc(hidden)]
    pub struct SpecialNonReactiveFuture<Fut> {
        #[pin]
        inner: Fut
    }
}

impl<Fut> SpecialNonReactiveFuture<Fut> {
    pub fn new(inner: Fut) -> Self {
        Self { inner }
    }
}

impl<Fut> Future for SpecialNonReactiveFuture<Fut>
where
    Fut: Future,
{
    type Output = Fut::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        #[cfg(debug_assertions)]
        let _rw = SpecialNonReactiveZone::enter();
        let this = self.project();
        this.inner.poll(cx)
    }
}

thread_local! {
    static SUPPRESS_RESOURCE_LOAD: Cell<bool> = const { Cell::new(false) };
}

#[doc(hidden)]
pub fn suppress_resource_load(suppress: bool) {
    SUPPRESS_RESOURCE_LOAD.with(|w| w.set(suppress));
}

#[doc(hidden)]
pub fn is_suppressing_resource_load() -> bool {
    SUPPRESS_RESOURCE_LOAD.with(|w| w.get())
}
