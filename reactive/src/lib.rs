#![feature(fn_traits)]
#![feature(let_chains)]
#![feature(unboxed_closures)]
#![feature(test)]

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

extern crate test;

#[cfg(test)]
mod tests {
    use test::Bencher;

    use std::{cell::Cell, rc::Rc};

    use crate::{create_effect, create_scope, create_signal};

    #[bench]
    fn create_and_update_1000_signals(b: &mut Bencher) {
        b.iter(|| {
            create_scope(|cx| {
                let acc = Rc::new(Cell::new(0));
                let sigs = (0..1000).map(|n| create_signal(cx, n)).collect::<Vec<_>>();
                assert_eq!(sigs.len(), 1000);
                /* create_effect(cx, {
                    let acc = Rc::clone(&acc);
                    move |_| {
                        for sig in &sigs {
                            acc.set(acc.get() + (sig.0)())
                        }
                    }
                });
                assert_eq!(acc.get(), 499500) */
            })
            .dispose()
        });
    }
}
