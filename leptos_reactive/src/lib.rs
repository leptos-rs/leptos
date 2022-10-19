#![cfg_attr(not(feature = "stable"), feature(fn_traits))]
#![cfg_attr(not(feature = "stable"), feature(unboxed_closures))]

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
//!    [WriteSignal](crate::WriteSignal)) tuple, or [create_rw_signal](crate::create_rw_signal), which returns
//!    a signal [RwSignal](crate::RwSignal) without this read-write segregation.
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
mod spawn;
mod suspense;

pub use context::*;
pub use effect::*;
pub use memo::*;

pub use resource::*;
use runtime::*;
pub use scope::*;
pub use selector::*;
pub use signal::*;
pub use spawn::*;
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
