#![feature(fn_traits)]
#![feature(unboxed_closures)]

mod context;
mod effect;
mod memo;
mod runtime;
mod scope;
mod signal;
mod source;
mod subscriber;
mod suspense;
mod transition;

pub use context::*;
pub use effect::*;
pub use memo::*;
use runtime::*;
pub use scope::*;
pub use signal::*;
use source::*;
use subscriber::*;
pub use suspense::*;
pub use transition::*;

#[macro_export]
macro_rules! debug_warn {
    ($($x:tt)*) => {
        {
            #[cfg(debug_assertions)]
            {
                log::warn!($($x)*)
            }
            #[cfg(not(debug_assertions))]
            { }
        }
    }
}
