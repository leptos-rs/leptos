#![feature(fn_traits)]
#![feature(unboxed_closures)]

mod computation;
mod context;
mod effect;
mod root_context;
mod scope;
mod signals;
mod suspense;
mod transition;

pub use computation::*;
pub use context::*;
pub use effect::*;
pub use root_context::*;
pub use scope::*;
pub use signals::*;
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
