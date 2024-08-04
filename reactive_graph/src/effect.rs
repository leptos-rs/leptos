//! Side effects that run in response to changes in the reactive values they read from.

#[allow(clippy::module_inception)]
mod effect;
mod inner;
mod render_effect;
pub use effect::*;
pub use render_effect::*;
