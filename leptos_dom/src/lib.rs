#![deny(missing_docs)]
#![forbid(unsafe_code)]

//! DOM helpers for Leptos.

/// DOM helpers — web-only. The macOS port does not provide an
/// equivalent module here; AppKit-flavoured app helpers live in
/// `cocoa_dom::app` instead.
#[cfg(feature = "web")]
pub mod helpers;
#[doc(hidden)]
#[cfg(feature = "web")]
pub mod macro_helpers;

/// Utilities for simple isomorphic logging to the console or terminal.
pub mod logging;
