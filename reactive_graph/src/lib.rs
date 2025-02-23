//! An implementation of a fine-grained reactive system.
//!
//! Fine-grained reactivity is an approach to modeling the flow of data through an interactive
//! application by composing together three categories of reactive primitives:
//! 1. **Signals**: atomic units of state, which can be directly mutated.
//! 2. **Computations**: derived values, which cannot be mutated directly but update whenever the signals
//!    they depend on change. These include both synchronous and asynchronous derived values.
//! 3. **Effects**: side effects that synchronize the reactive system with the non-reactive world
//!    outside it.
//!
//! Signals and computations are "source" nodes in the reactive graph, because an observer can
//! subscribe to them to respond to changes in their values. Effects and computations are "subscriber"
//! nodes, because they can listen to changes in other values.
//!
//! ```rust
//! # any_spawner::Executor::init_futures_executor();
//! # let owner = reactive_graph::owner::Owner::new(); owner.set();
//! use reactive_graph::{
//!     computed::ArcMemo,
//!     effect::Effect,
//!     prelude::{Read, Set},
//!     signal::ArcRwSignal,
//! };
//!
//! let count = ArcRwSignal::new(1);
//! let double_count = ArcMemo::new({
//!     let count = count.clone();
//!     move |_| *count.read() * 2
//! });
//!
//! // the effect will run once initially
//! Effect::new(move |_| {
//!     println!("double_count = {}", *double_count.read());
//! });
//!
//! // updating `count` will propagate changes to the dependencies,
//! // causing the effect to run again
//! count.set(2);
//! ```
//!
//! This reactivity is called "fine grained" because updating the value of a signal only affects
//! the effects and computations that depend on its value, without requiring any diffing or update
//! calculations for other values.
//!
//! This model is especially suitable for building user interfaces, i.e., long-lived systems in
//! which changes can begin from many different entry points. It is not particularly useful in
//! "run-once" programs like a CLI.
//!
//! ## Design Principles and Assumptions
//! - **Effects are expensive.** The library is built on the assumption that the side effects
//!   (making a network request, rendering something to the DOM, writing to disk) are orders of
//!   magnitude more expensive than propagating signal updates. As a result, the algorithm is
//!   designed to avoid re-running side effects unnecessarily, and is willing to sacrifice a small
//!   amount of raw update speed to that goal.
//! - **Automatic dependency tracking.** Dependencies are not specified as a compile-time list, but
//!   tracked at runtime. This in turn enables **dynamic dependency tracking**: subscribers
//!   unsubscribe from their sources between runs, which means that a subscriber that contains a
//!   condition branch will not re-run when dependencies update that are only used in the inactive
//!   branch.
//! - **Asynchronous effect scheduling.** Effects are spawned as asynchronous tasks. This means
//!   that while updating a signal will immediately update its value, effects that depend on it
//!   will not run until the next "tick" of the async runtime. (This in turn means that the
//!   reactive system is *async runtime agnostic*: it can be used in the browser with
//!   `wasm-bindgen-futures`, in a native binary with `tokio`, in a GTK application with `glib`,
//!   etc.)
//!
//! The reactive-graph algorithm used in this crate is based on that of
//! [Reactively](https://github.com/modderme123/reactively), as described
//! [in this article](https://dev.to/modderme123/super-charging-fine-grained-reactive-performance-47ph).

#![cfg_attr(feature = "nightly", feature(unboxed_closures))]
#![cfg_attr(feature = "nightly", feature(fn_traits))]
#![deny(missing_docs)]

use std::{fmt::Arguments, future::Future};

pub mod actions;
pub(crate) mod channel;
pub mod computed;
pub mod diagnostics;
pub mod effect;
pub mod graph;
pub mod owner;
#[cfg(feature = "serde")]
mod serde;
pub mod signal;
mod trait_options;
pub mod traits;
pub mod transition;
pub mod wrappers;

use computed::ScopedFuture;

#[cfg(feature = "nightly")]
mod nightly;

/// Reexports frequently-used traits.
pub mod prelude {
    pub use crate::{owner::FromLocal, traits::*};
}

// TODO remove this, it's just useful while developing
#[allow(unused)]
#[doc(hidden)]
pub fn log_warning(text: Arguments) {
    #[cfg(feature = "tracing")]
    {
        tracing::warn!(text);
    }
    #[cfg(all(
        not(feature = "tracing"),
        target_arch = "wasm32",
        target_os = "unknown"
    ))]
    {
        web_sys::console::warn_1(&text.to_string().into());
    }
    #[cfg(all(
        not(feature = "tracing"),
        not(all(target_arch = "wasm32", target_os = "unknown"))
    ))]
    {
        eprintln!("{}", text);
    }
}

/// Calls [`Executor::spawn`](any_spawner::Executor), but ensures that the task also runs in the current arena, if
/// multithreaded arena sandboxing is enabled.
pub fn spawn(task: impl Future<Output = ()> + Send + 'static) {
    #[cfg(feature = "sandboxed-arenas")]
    let task = owner::Sandboxed::new(task);

    any_spawner::Executor::spawn(task);
}

/// Calls [`Executor::spawn_local`](any_spawner::Executor), but ensures that the task runs under the current reactive [`Owner`](crate::owner::Owner) and observer.
///
/// Does not cancel the task if the owner is cleaned up.
pub fn spawn_local_scoped(task: impl Future<Output = ()> + 'static) {
    let task = ScopedFuture::new(task);

    #[cfg(feature = "sandboxed-arenas")]
    let task = owner::Sandboxed::new(task);

    any_spawner::Executor::spawn_local(task);
}

/// Calls [`Executor::spawn_local`](any_spawner::Executor), but ensures that the task runs under the current reactive [`Owner`](crate::owner::Owner) and observer.
///
/// Cancels the task if the owner is cleaned up.
pub fn spawn_local_scoped_with_cancellation(
    task: impl Future<Output = ()> + 'static,
) {
    use crate::owner::on_cleanup;
    use futures::future::{AbortHandle, Abortable};

    let (abort_handle, abort_registration) = AbortHandle::new_pair();
    on_cleanup(move || abort_handle.abort());

    let task = Abortable::new(task, abort_registration);
    let task = ScopedFuture::new(task);

    #[cfg(feature = "sandboxed-arenas")]
    let task = owner::Sandboxed::new(task);

    any_spawner::Executor::spawn_local(async move {
        _ = task.await;
    });
}
