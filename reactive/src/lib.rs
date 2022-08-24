#![feature(fn_traits)]
#![feature(let_chains)]
#![feature(unboxed_closures)]

// The implementation of this reactive system is largely a Rust port of [Flimsy](https://github.com/fabiospampinato/flimsy/blob/master/src/flimsy.annotated.ts),
// which is itself a simplified and annotated version of SolidJS reactivity.

mod computation;
mod context;
mod memo;
mod resource;
mod scope;
mod signal;
mod spawn;
mod suspense;
mod system;
mod transition;

pub use computation::*;
pub use context::*;
pub use memo::*;
pub use resource::*;
pub use scope::*;
pub use signal::*;
pub use spawn::*;
pub use suspense::*;
pub use system::*;
pub use transition::*;
