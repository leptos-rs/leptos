#![deny(missing_docs)]
#![cfg_attr(feature = "nightly", feature(fn_traits))]
#![cfg_attr(feature = "nightly", feature(unboxed_closures))]
#![cfg_attr(feature = "nightly", feature(type_name_of_val))]

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
//! 1. *Signals:* [`create_signal`](crate::create_signal), which returns a ([`ReadSignal`](crate::ReadSignal),
//!    [`WriteSignal`](crate::WriteSignal)) tuple, or [`create_rw_signal`](crate::create_rw_signal), which returns
//!    a signal [`RwSignal`](crate::RwSignal) without this read-write segregation.
//! 2. *Derived Signals:* any function that relies on another signal.
//! 3. *Memos:* [`create_memo`], which returns a [`Memo`](crate::Memo).
//! 4. *Resources:* [`create_resource`], which converts an `async` [`Future`](std::future::Future) into a
//!    synchronous [`Resource`](crate::Resource) signal.
//! 5. *Triggers:* [`create_trigger`], creates a purely reactive [`Trigger`] primitive without any associated state.
//!
//! ### Effects
//! 1. Use [`create_effect`](crate::create_effect) when you need to synchronize the reactive system
//!    with something outside it (for example: logging to the console, writing to a file or local storage)
//! 2. The Leptos DOM renderer wraps any [`Fn`] in your template with [`create_effect`](crate::create_effect), so
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
//!     // a signal: returns a (getter, setter) pair
//!     let (count, set_count) = create_signal(cx, 0);
//!
//!     // calling the getter gets the value
//!     // can be `count()` on nightly
//!     assert_eq!(count.get(), 0);
//!     // calling the setter sets the value
//!     // can be `set_count(1)` on nightly
//!     set_count.set(1);
//!     // or we can mutate it in place with update()
//!     set_count.update(|n| *n += 1);
//!
//!     // a derived signal: a plain closure that relies on the signal
//!     // the closure will run whenever we *access* double_count()
//!     let double_count = move || count.get() * 2;
//!     assert_eq!(double_count(), 4);
//!
//!     // a memo: subscribes to the signal
//!     // the closure will run only when count changes
//!     let memoized_triple_count = create_memo(cx, move |_| count.get() * 3);
//!     // can be `memoized_triple_count()` on nightly
//!     assert_eq!(memoized_triple_count.get(), 6);
//!
//!     // this effect will run whenever `count` changes
//!     create_effect(cx, move |_| {
//!         println!("Count = {}", count.get());
//!     });
//! });
//! ```

#[cfg_attr(any(debug_assertions, feature = "ssr"), macro_use)]
extern crate tracing;

#[macro_use]
mod signal;
mod context;
#[macro_use]
mod diagnostics;
mod effect;
mod hydration;
mod memo;
mod node;
mod resource;
mod runtime;
mod scope;
mod selector;
mod serialization;
mod signal_wrappers_read;
mod signal_wrappers_write;
mod slice;
mod spawn;
mod spawn_microtask;
mod stored_value;
pub mod suspense;
mod trigger;
mod watch;

pub use context::*;
pub use diagnostics::SpecialNonReactiveZone;
pub use effect::*;
pub use hydration::FragmentData;
pub use memo::*;
pub use resource::*;
use runtime::*;
pub use runtime::{create_runtime, RuntimeId};
pub use scope::*;
pub use selector::*;
pub use serialization::*;
pub use signal::{prelude as signal_prelude, *};
pub use signal_wrappers_read::*;
pub use signal_wrappers_write::*;
pub use slice::*;
pub use spawn::*;
pub use spawn_microtask::*;
pub use stored_value::*;
pub use suspense::{GlobalSuspenseContext, SuspenseContext};
pub use trigger::*;
pub use watch::*;

mod macros {
    macro_rules! debug_warn {
        ($($x:tt)*) => {
            {
                #[cfg(debug_assertions)]
                {
                    ($crate::console_warn(&format_args!($($x)*).to_string()))
                }
                #[cfg(not(debug_assertions))]
                {
                    ($($x)*)
                }
            }
        }
    }

    pub(crate) use debug_warn;
}

pub(crate) fn console_warn(s: &str) {
    cfg_if::cfg_if! {
        if #[cfg(all(target_arch = "wasm32", any(feature = "csr", feature = "hydrate")))] {
            web_sys::console::warn_1(&wasm_bindgen::JsValue::from_str(s));
        } else {
            eprintln!("{s}");
        }
    }
}
