// The point of these diagnostics is to give useful error messages when someone
// tries to access a reactive variable outside the reactive scope. They track when
// you create a signal/memo, and where you access it non-reactively.

#[cfg(debug_assertions)]
#[allow(dead_code)] // allowed for SSR
#[derive(Copy, Clone)]
pub(crate) struct AccessDiagnostics {
    pub defined_at: &'static std::panic::Location<'static>,
    pub called_at: &'static std::panic::Location<'static>,
}

#[cfg(not(debug_assertions))]
#[derive(Copy, Clone, Default)]
pub(crate) struct AccessDiagnostics;

/// This just tracks whether we're currently in a context in which it really doesn't
/// matter whether something is reactive: for example, in an event listener or timeout.
/// Entering this zone basically turns off the warnings, and exiting it turns them back on.
/// All of this is a no-op in release mode.
#[doc(hidden)]
#[derive(Debug)]
pub struct SpecialNonReactiveZone;

/// Exits the "special non-reactive zone" when dropped.
#[derive(Debug)]
pub struct SpecialNonReactiveZoneGuard;

use std::cell::Cell;

thread_local! {
    static IS_SPECIAL_ZONE: Cell<bool> = const { Cell::new(false) };
}

impl SpecialNonReactiveZone {
    // TODO: the fact that this is unused probably means we haven't set diagnostics up at all
    // we should do that! (i.e., warn if you're doing a reactive access with no owner but you're not
    // inside a special zone)
    #[inline(always)]
    pub(crate) fn is_inside() -> bool {
        if cfg!(debug_assertions) {
            IS_SPECIAL_ZONE.get()
        } else {
            false
        }
    }

    pub fn enter() -> SpecialNonReactiveZoneGuard {
        IS_SPECIAL_ZONE.set(true);
        SpecialNonReactiveZoneGuard
    }
}

impl Drop for SpecialNonReactiveZoneGuard {
    fn drop(&mut self) {
        IS_SPECIAL_ZONE.set(false);
    }
}

#[doc(hidden)]
#[macro_export]
macro_rules! diagnostics {
    ($this:ident) => {{
        #[cfg(debug_assertions)]
        {
            AccessDiagnostics {
                defined_at: $this.defined_at,
                called_at: std::panic::Location::caller(),
            }
        }
        #[cfg(not(debug_assertions))]
        {
            AccessDiagnostics
        }
    }};
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
