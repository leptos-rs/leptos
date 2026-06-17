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
pub struct LazyViewError(SendWrapper<SplitLoaderError>);

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

impl std::error::Error for LazyViewError {}

impl From<SplitLoaderError> for LazyViewError {
    fn from(err: SplitLoaderError) -> Self {
        Self(SendWrapper::new(err))
    }
}
