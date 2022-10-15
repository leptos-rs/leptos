#![feature(fn_traits)]
#![feature(let_chains)]
#![feature(unboxed_closures)]
#![feature(test)]

//! The reactive system for the [Leptos](https://docs.rs/leptos/latest/leptos/) Web framework.
//!
//! ## Fine-Grained Reactivity
//!
//! Leptos is built on a fine-grained reactive system, which means that individual reactive values
//! (“signals,” sometimes known as observables) trigger the code that reacts to them (“effects,”
//! sometimes known as observers) to re-run. These two halves of the reactive system are inter-dependent.
//! Without effects, signals can change within the reactive system but never be observed in a way
//! that interacts with the outside world. Without signals, effects run once but never again, as
//! there’s no observable value to subscribe to.
//!
//! Here are the most commonly-used functions and types you'll need to build a reactive system:
//!
//! ### Signals
//! 1. *Signals:* [create_signal](crate::create_signal), which returns a ([ReadSignal](crate::ReadSignal),
//!    [WriteSignal](crate::WriteSignal)) tuple.
//! 2. *Derived Signals:* any function that relies on another signal.
//! 3. *Memos:* [create_memo](crate::create_memo), which returns a [Memo](crate::Memo).
//! 4. *Resources:* [create_resource], which converts an `async` [Future] into a synchronous [Resource](crate::Resource) signal.
//!
//! ### Effects
//! 1. Use [create_effect](crate::create_effect) when you need to synchronize the reactive system
//!    with something outside it (for example: logging to the console, writing to a file or local storage)
//! 2. The Leptos DOM renderer wraps any [Fn] in your template with [create_effect](crate::create_effect), so
//!    components you write do *not* need explicit effects to synchronize with the DOM.
//!
//! ### Example
//! ```
//! use leptos_reactive::*;
//!
//! // creates a new reactive Scope
//! // this is omitted from most of the examples in the docs
//! // you usually won't need to call it yourself
//! create_scope(|cx| {
//!   // a signal: returns a (getter, setter) pair
//!   let (count, set_count) = create_signal(cx, 0);
//!
//!   // calling the getter gets the value
//!   assert_eq!(count(), 0);
//!   // calling the setter sets the value
//!   set_count(1);
//!   // or we can mutate it in place with update()
//!   set_count.update(|n| *n += 1);
//!
//!   // a derived signal: a plain closure that relies on the signal
//!   // the closure will run whenever we *access* double_count()
//!   let double_count = move || count() * 2;
//!   assert_eq!(double_count(), 4);
//!   
//!   // a memo: subscribes to the signal
//!   // the closure will run only when count changes
//!   let memoized_triple_count = create_memo(cx, move |_| count() * 3);
//!   assert_eq!(memoized_triple_count(), 6);
//!
//!   // this effect will run whenever count() changes
//!   create_effect(cx, move |_| {
//!     println!("Count = {}", count());
//!   });
//! });
//! ```

mod context;
mod effect;
mod hydration;
mod memo;
mod resource;
mod runtime;
mod scope;
mod selector;
mod signal;
mod source;
mod spawn;
mod subscriber;
mod suspense;

pub use context::*;
pub use effect::*;
pub use memo::*;
pub use resource::*;
use runtime::*;
pub use scope::*;
pub use selector::*;
pub use signal::*;
use source::*;
pub use spawn::*;
use subscriber::*;
pub use suspense::*;

#[doc(hidden)]
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

    #[bench]
    fn create_and_update_1000_signals(b: &mut Bencher) {
        use crate::{create_effect, create_memo, create_scope, create_signal};

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

    #[bench]
    fn create_and_dispose_1000_scopes(b: &mut Bencher) {
        use crate::{create_effect, create_scope, create_signal};

        b.iter(|| {
            let acc = Rc::new(Cell::new(0));
            let disposers = (0..1000)
                .map(|_| {
                    create_scope({
                        let acc = Rc::clone(&acc);
                        move |cx| {
                            let (r, w) = create_signal(cx, 0);
                            create_effect(cx, {
                                move |_| {
                                    acc.set(r());
                                }
                            });
                            w.update(|n| *n += 1);
                        }
                    })
                })
                .collect::<Vec<_>>();
            for disposer in disposers {
                disposer.dispose();
            }
        });
    }

    #[bench]
    fn sycamore_create_and_update_1000_signals(b: &mut Bencher) {
        use sycamore::reactive::{create_effect, create_memo, create_scope, create_signal};

        b.iter(|| {
            let d = create_scope(|cx| {
                let acc = Rc::new(Cell::new(0));
                let sigs = Rc::new((0..1000).map(|n| create_signal(cx, n)).collect::<Vec<_>>());
                let memo = create_memo(cx, {
                    let sigs = Rc::clone(&sigs);
                    move || sigs.iter().map(|r| *r.get()).sum::<i32>()
                });
                assert_eq!(*memo.get(), 499500);
                create_effect(cx, {
                    let acc = Rc::clone(&acc);
                    move || {
                        acc.set(*memo.get());
                    }
                });
                assert_eq!(acc.get(), 499500);

                sigs[1].set(*sigs[1].get() + 1);
                sigs[10].set(*sigs[10].get() + 1);
                sigs[100].set(*sigs[100].get() + 1);

                assert_eq!(acc.get(), 499503);
                assert_eq!(*memo.get(), 499503);
            });
            unsafe { d.dispose() };
        });
    }

    #[bench]
    fn sycamore_create_and_dispose_1000_scopes(b: &mut Bencher) {
        use sycamore::reactive::{create_effect, create_scope, create_signal};

        b.iter(|| {
            let acc = Rc::new(Cell::new(0));
            let disposers = (0..1000)
                .map(|_| {
                    create_scope({
                        let acc = Rc::clone(&acc);
                        move |cx| {
                            let s = create_signal(cx, 0);
                            create_effect(cx, {
                                move || {
                                    acc.set(*s.get());
                                }
                            });
                            s.set(*s.get() + 1);
                        }
                    })
                })
                .collect::<Vec<_>>();
            for disposer in disposers {
                unsafe {
                    disposer.dispose();
                }
            }
        });
    }
}
