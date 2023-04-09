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
pub(crate) struct AccessDiagnostics {}

/// This just tracks whether we're currently in a context in which it really doesn't
/// matter whether something is reactive: for example, in an event listener or timeout.
/// Entering this zone basically turns off the warnings, and exiting it turns them back on.
/// All of this is a no-op in release mode.
#[doc(hidden)]
pub struct SpecialNonReactiveZone {}

cfg_if::cfg_if! {
    if #[cfg(debug_assertions)] {
        use std::cell::Cell;

        thread_local! {
            static IS_SPECIAL_ZONE: Cell<bool> = Cell::new(false);
        }
    }
}

impl SpecialNonReactiveZone {
    #[allow(dead_code)] // allowed for SSR
    #[inline(always)]
    pub(crate) fn is_inside() -> bool {
        #[cfg(debug_assertions)]
        {
            IS_SPECIAL_ZONE.with(|val| val.get())
        }
        #[cfg(not(debug_assertions))]
        false
    }

    #[inline(always)]
    pub fn enter() {
        #[cfg(debug_assertions)]
        {
            IS_SPECIAL_ZONE.with(|val| val.set(true))
        }
    }

    #[inline(always)]
    pub fn exit() {
        #[cfg(debug_assertions)]
        {
            IS_SPECIAL_ZONE.with(|val| val.set(false))
        }
    }
}

#[doc(hidden)]
#[macro_export]
macro_rules! diagnostics {
    ($this:ident) => {{
        cfg_if::cfg_if! {
            if #[cfg(debug_assertions)] {
                AccessDiagnostics {
                    defined_at: $this.defined_at,
                    called_at: std::panic::Location::caller()
                }
            } else {
                AccessDiagnostics { }
            }
        }
    }};
}
