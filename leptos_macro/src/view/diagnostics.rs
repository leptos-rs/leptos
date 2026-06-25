//! A small, centralized diagnostics layer for the `view!`/`template!` macros.
//!
//! Lowering code records diagnostics as *values* (via [`error`]) instead of
//! calling `proc_macro_error2::emit_error!`/`abort!` directly at the point of
//! failure. A single sink at the macro boundary ([`emit_all`]) drains the
//! buffer and reports everything at once.
//!
//! This buys three things the previous ad-hoc calls could not:
//!
//! 1. **Locality by construction** – every diagnostic must be handed an
//!    explicit [`Span`]. `Span::call_site()` (which underlines the *entire*
//!    `view!` invocation) is reserved for the single documented overflow
//!    summary below, so it can no longer leak in by accident.
//! 2. **Bounded output** – identical diagnostics are de-duplicated and the
//!    total is capped at [`MAX_DIAGNOSTICS`], so one mistake can't bury the
//!    user under a wall of repeated/cascading errors.
//! 3. **Testability** – [`collect`] runs lowering and returns the recorded
//!    diagnostics *without* touching the global `proc_macro_error` sink, which
//!    is what lets us unit-test error *locality* (see `view/tests.rs`).
//!
//! Only errors are modelled today (every `view!` diagnostic is one); a warning
//! level can be added the same way if a use case appears.

use proc_macro2::Span;
use std::cell::RefCell;

/// Maximum number of distinct diagnostics surfaced from a single macro
/// invocation. Anything beyond this is collapsed into one summary message so a
/// single mistake cannot flood the user with hundreds of (often cascading)
/// errors.
pub const MAX_DIAGNOSTICS: usize = 16;

/// A single deferred error: a message anchored at a specific [`Span`].
pub struct Diagnostic {
    pub span: Span,
    pub message: String,
}

impl Diagnostic {
    /// Key used to collapse duplicate diagnostics. Two are considered the same
    /// iff they share a message and a span (compared by its debug
    /// representation, which encodes the source range).
    fn dedup_key(&self) -> (String, String) {
        (self.message.clone(), format!("{:?}", self.span))
    }
}

thread_local! {
    static BUFFER: RefCell<Vec<Diagnostic>> = const { RefCell::new(Vec::new()) };
}

/// Record an error anchored at `span`, to be emitted at the macro boundary.
pub fn error(span: Span, message: impl Into<String>) {
    let message = message.into();
    BUFFER
        .with(|buffer| buffer.borrow_mut().push(Diagnostic { span, message }));
}

/// Drain the buffer, applying de-dup and the [`MAX_DIAGNOSTICS`] cap.
fn take_normalized() -> Vec<Diagnostic> {
    let raw = BUFFER.with(|buffer| std::mem::take(&mut *buffer.borrow_mut()));

    let mut seen = std::collections::HashSet::new();
    let mut out: Vec<Diagnostic> = Vec::new();
    let mut suppressed = 0usize;

    for diagnostic in raw {
        if !seen.insert(diagnostic.dedup_key()) {
            continue; // exact duplicate
        }
        if out.len() >= MAX_DIAGNOSTICS {
            suppressed += 1;
            continue;
        }
        out.push(diagnostic);
    }

    if suppressed > 0 {
        // The only sanctioned use of `call_site()`: a summary that is not tied
        // to any single token by nature.
        out.push(Diagnostic {
            span: Span::call_site(),
            message: format!(
                "{suppressed} additional view! diagnostic(s) were suppressed; \
                 fix the errors above and recompile to see the rest"
            ),
        });
    }

    out
}

/// Drain and emit every buffered diagnostic through `proc_macro_error2`.
///
/// Must be called exactly once, at the macro boundary, from within a
/// `#[proc_macro_error]` function.
pub fn emit_all() {
    for Diagnostic { span, message } in take_normalized() {
        // Format with an explicit `{}` so braces inside `message` are never
        // interpreted as format directives.
        proc_macro_error2::emit_error!(span, "{}", message);
    }
}

/// Run `f`, returning its result together with the diagnostics it recorded,
/// *without* emitting them. Used by the locality unit tests.
#[cfg(test)]
pub fn collect<T>(f: impl FnOnce() -> T) -> (T, Vec<Diagnostic>) {
    // Start from a clean slate so leftover state from a previous call on this
    // thread can't contaminate the result.
    let _ = BUFFER.with(|buffer| std::mem::take(&mut *buffer.borrow_mut()));
    let out = f();
    let diagnostics = take_normalized();
    (out, diagnostics)
}
