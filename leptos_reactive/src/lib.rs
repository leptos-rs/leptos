#![deny(missing_docs)]
#![cfg_attr(not(feature = "stable"), feature(fn_traits))]
#![cfg_attr(not(feature = "stable"), feature(unboxed_closures))]
#![cfg_attr(not(feature = "stable"), feature(type_name_of_val))]

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
//! 4. *Resources:* [create_resource], which converts an `async` [std::future::Future] into a
//!    synchronous [Resource](crate::Resource) signal.
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
//! create_scope(create_runtime(), |cx| {
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

#[cfg_attr(debug_assertions, macro_use)]
pub extern crate tracing;

mod context;
mod effect;
mod hydration;
mod memo;
mod resource;
mod runtime;
mod scope;
mod selector;
mod serialization;
mod signal;
mod signal_wrappers_read;
mod signal_wrappers_write;
mod slice;
mod spawn;
mod spawn_microtask;
mod stored_value;
mod suspense;

pub use context::*;
pub use effect::*;
pub use memo::*;
pub use resource::*;
use runtime::*;
pub use runtime::{create_runtime, RuntimeId};
pub use scope::*;
pub use selector::*;
pub use serialization::*;
pub use signal::*;
pub use signal_wrappers_read::*;
pub use signal_wrappers_write::*;
pub use slice::*;
pub use spawn::*;
pub use spawn_microtask::*;
pub use stored_value::*;
pub use suspense::*;

/// Trait implemented for all signal types which you can `get` a value
/// from, such as [`ReadSignal`],
/// [`Memo`], etc., which allows getting the inner value without
/// subscribing to the current scope.
pub trait UntrackedGettableSignal<T> {
    /// Gets the signal's value without creating a dependency on the
    /// current scope.
    fn get_untracked(&self) -> T
    where
        T: Clone;

    /// Runs the provided closure with a reference to the current
    /// value without creating a dependency on the current scope.
    fn with_untracked<O>(&self, f: impl FnOnce(&T) -> O) -> O;
}

/// Trait implemented for all signal types which you can `set` the inner
/// value, such as [`WriteSignal`] and [`RwSignal`], which allows setting
/// the inner value without causing effects which depend on the signal
/// from being run.
pub trait UntrackedSettableSignal<T> {
    /// Sets the signal's value without notifying dependents.
    fn set_untracked(&self, new_value: T);

    /// Runs the provided closure with a mutable reference to the current
    /// value without notifying dependents.
    fn update_untracked(&self, f: impl FnOnce(&mut T));

    /// Runs the provided closure with a mutable reference to the current
    /// value without notifying dependents and returns
    /// the value the closure returned.
    fn update_returning_untracked<U>(&self, f: impl FnOnce(&mut T) -> U) -> Option<U>;
}

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
