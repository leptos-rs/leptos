//! Global request body size limit applied by server function handlers.
//!
//! The Axum and Actix server function integrations buffer request bodies into
//! memory before decoding. Unlimited-size uploads can exhaust server memory,
//! so this module provides a configurable global upper limit.
//!
//! The limits do *not* apply to streaming request bodies like multipart data.
//!
//! The default matches Axum's default `Bytes`/`Json` extractor limit
//! ([`DEFAULT_BODY_LIMIT_BYTES`], 2 MiB).

#![allow(unused)]

use std::sync::atomic::{AtomicUsize, Ordering};

/// Default maximum buffered request body size, in bytes (2 MiB).
///
/// Matches Axum's default body limit.
pub const DEFAULT_BODY_LIMIT_BYTES: usize = 2 * 1024 * 1024;

static BODY_LIMIT: AtomicUsize = AtomicUsize::new(DEFAULT_BODY_LIMIT_BYTES);

/// The maximum request body size, in bytes, applied by server function handlers.
///
/// This limit does not apply to streaming or multipart requests.
pub fn default_body_limit() -> usize {
    BODY_LIMIT.load(Ordering::Relaxed)
}

/// Sets the maximum buffered request body size, in bytes, applied by server
/// function handlers.
pub fn set_default_body_limit(limit: usize) {
    BODY_LIMIT.store(limit, Ordering::Relaxed);
}
