//! Error type bridging a failed lazy-route chunk load into an
//! [`ErrorBoundary`](crate::error::ErrorBoundary).

use send_wrapper::SendWrapper;
use std::fmt;
use wasm_split_helpers::SplitLoaderError;

/// The error produced when a lazy route's view fails to load its WASM chunk.
///
/// `#[lazy_route]` generates a `fallible` view whose `Err` is rendered by the
/// nearest [`ErrorBoundary`](crate::error::ErrorBoundary) instead of panicking.
/// That rendering path requires the error to be `Send + Sync + 'static`, but the
/// underlying [`SplitLoaderError`] is deliberately allowed to be neither (so it
/// can, for example, carry a `JsValue` describing the JavaScript failure).
///
/// This wrapper bridges the two: it holds the split error in a [`SendWrapper`],
/// which is `Send + Sync` unconditionally. That is sound here because a lazy
/// chunk only ever loads — and so this error is only ever constructed and
/// rendered — on the single WASM thread.
///
/// You can match on this type from an `<ErrorBoundary>` fallback (via
/// `Error::downcast_ref`) to detect a chunk-load failure specifically.
#[derive(Clone)]
pub struct LazyViewError(SendWrapper<SplitLoaderError>);

// Delegate `Debug`/`Display` to the inner error rather than deriving them, so
// diagnostics show the `SplitLoaderError` itself and not the `SendWrapper`
// implementation detail.
impl fmt::Debug for LazyViewError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&*self.0, f)
    }
}

impl fmt::Display for LazyViewError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&*self.0, f)
    }
}

impl std::error::Error for LazyViewError {
    // Preserve the chain to the underlying `SplitLoaderError` so a fallback can
    // walk `source()` (or downcast it) for the load-failure details.
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&*self.0)
    }
}

impl From<SplitLoaderError> for LazyViewError {
    fn from(err: SplitLoaderError) -> Self {
        Self(SendWrapper::new(err))
    }
}
