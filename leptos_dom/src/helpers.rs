//! DOM helpers for Leptos.

#[cfg(not(target_os = "wasi"))]
#[path = "helpers_browser.rs"]
mod helpers_browser;
#[cfg(not(target_os = "wasi"))]
pub use helpers_browser::*;

#[cfg(target_os = "wasi")]
#[path = "helpers_wasi.rs"]
mod helpers_wasi;
#[cfg(target_os = "wasi")]
pub use helpers_wasi::*;
