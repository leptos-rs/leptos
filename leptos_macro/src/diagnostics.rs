//! Diagnostics collected while lowering macro input.
//!
//! Lowering code records errors as *values* in an [`Errors`] accumulator
//! instead of aborting at the point of failure, and the macro boundary reports
//! everything at once via [`Errors::finish`].
//!
//! This buys a few things ad-hoc aborts could not:
//!
//! 1. **Locality by construction** – every diagnostic is a [`syn::Error`]
//!    anchored at an explicit span. `Span::call_site()` (which underlines the
//!    *entire* macro invocation) is reserved for the single documented overflow
//!    summary below.
//! 2. **Bounded output** – identical diagnostics are de-duplicated and the
//!    total is capped at [`MAX_DIAGNOSTICS`], so one mistake can't bury the
//!    user under a wall of repeated/cascading errors.
//! 3. **Testability** – lowering returns its diagnostics as a plain
//!    `syn::Result`, which is what lets us unit-test error *locality* (see
//!    `view/tests.rs`).

use proc_macro2::Span;
use std::collections::HashSet;

/// Maximum number of distinct diagnostics surfaced from a single macro
/// invocation. Anything beyond this is collapsed into one summary message so a
/// single mistake cannot flood the user with hundreds of (often cascading)
/// errors.
pub(crate) const MAX_DIAGNOSTICS: usize = 16;

#[derive(Default)]
pub(crate) struct Errors(Vec<syn::Error>);

impl Errors {
    pub(crate) fn push(&mut self, error: syn::Error) {
        self.0.push(error);
    }

    pub(crate) fn finish<T>(
        mut self,
        result: syn::Result<T>,
    ) -> syn::Result<T> {
        match result {
            Ok(value) if self.0.is_empty() => Ok(value),
            Ok(_) => Err(self.normalize()),
            Err(error) => {
                self.0.push(error);
                Err(self.normalize())
            }
        }
    }

    /// Combines every recorded error into one, applying de-dup and the
    /// [`MAX_DIAGNOSTICS`] cap.
    fn normalize(self) -> syn::Error {
        let mut seen = HashSet::new();
        let mut out: Option<syn::Error> = None;
        let mut count = 0usize;
        let mut suppressed = 0usize;

        // A single `syn::Error` may already carry several messages (e.g. one
        // propagated from a nested `Errors`), so flatten before counting.
        for error in self.0.into_iter().flatten() {
            // Two diagnostics are the same iff they share a message and a
            // span (compared by its debug representation, which encodes the
            // source range).
            let key = (error.to_string(), format!("{:?}", error.span()));
            if !seen.insert(key) {
                continue; // exact duplicate
            }
            if count >= MAX_DIAGNOSTICS {
                suppressed += 1;
                continue;
            }
            count += 1;
            match &mut out {
                Some(existing) => existing.combine(error),
                None => out = Some(error),
            }
        }

        let mut out = out.expect("normalize is only called with errors");
        if suppressed > 0 {
            // The only sanctioned use of `call_site()`: a summary that is not
            // tied to any single token by nature.
            out.combine(syn::Error::new(
                Span::call_site(),
                format!(
                    "{suppressed} additional diagnostic(s) were suppressed; \
                     fix the errors above and recompile to see the rest"
                ),
            ));
        }
        out
    }
}

pub(crate) fn message_with_help(
    message: impl std::fmt::Display,
    help: impl std::fmt::Display,
) -> String {
    format!("{message}\n\n  = help: {help}\n\n")
}
