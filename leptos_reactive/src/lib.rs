#![feature(fn_traits)]
#![feature(let_chains)]
#![feature(unboxed_closures)]
#![feature(test)]

mod context;
mod effect;
mod memo;
mod resource;
mod runtime;
mod scope;
mod signal;
mod source;
mod spawn;
mod subscriber;
mod suspense;
mod transition;

pub use context::*;
pub use effect::*;
pub use memo::*;
pub use resource::*;
use runtime::*;
pub use scope::*;
pub use signal::*;
use source::*;
use spawn::*;
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

extern crate test;

#[cfg(test)]
mod tests {
    use test::Bencher;

    use std::{cell::Cell, rc::Rc};

    use crate::{create_effect, create_memo, create_scope, create_signal};

    #[bench]
    fn create_and_update_1000_signals(b: &mut Bencher) {
        b.iter(|| {
            create_scope(|cx| {
                let acc = Rc::new(Cell::new(0));
                let sigs = (0..1000).map(|n| create_signal(cx, n)).collect::<Vec<_>>();
                let reads = sigs.iter().map(|(r, _)| *r).collect::<Vec<_>>();
                let writes = sigs.iter().map(|(_, w)| *w).collect::<Vec<_>>();
                let memo = create_memo(cx, move |_| reads.iter().map(|r| r.get()).sum::<i32>());
                assert_eq!(memo(), 499500);
                create_effect(cx, {
                    let acc = Rc::clone(&acc);
                    move |_| {
                        acc.set(memo());
                    }
                });
                assert_eq!(acc.get(), 499500);

                writes[1].update(|n| *n += 1);
                writes[10].update(|n| *n += 1);
                writes[100].update(|n| *n += 1);

                assert_eq!(acc.get(), 499503);
                assert_eq!(memo(), 499503);
            })
            .dispose()
        });
    }
}
