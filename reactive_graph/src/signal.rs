//! Reactive primitives for root values that can be changed, notifying other nodes in the reactive
//! graph.

mod arc_read;
mod arc_rw;
mod arc_trigger;
mod arc_write;
pub mod guards;
mod read;
mod rw;
mod subscriber_traits;
mod trigger;
mod write;

use crate::owner::LocalStorage;
pub use arc_read::*;
pub use arc_rw::*;
pub use arc_trigger::*;
pub use arc_write::*;
pub use read::*;
pub use rw::*;
pub use trigger::*;
pub use write::*;

/// Creates a reference-counted signal.
///
/// A signal is a piece of data that may change over time, and notifies other
/// code when it has changed. This is the atomic unit of reactivity, which begins all other
/// processes of updating.
///
/// Takes the initial value as an argument, and returns a tuple containing an
/// [`ArcReadSignal`] and an [`ArcWriteSignal`].
///
/// This returns reference-counted signals, which are `Clone` but not `Copy`. For arena-allocated
/// `Copy` signals, use [`signal`].
///
/// ```
/// # use reactive_graph::prelude::*;
/// # use reactive_graph::signal::*; let owner = reactive_graph::owner::Owner::new(); owner.set();
/// let (count, set_count) = arc_signal(0);
///
/// // ✅ calling the getter clones and returns the value
/// //    this can be `count()` on nightly
/// assert_eq!(count.get(), 0);
///
/// // ✅ calling the setter sets the value
/// //    this can be `set_count(1)` on nightly
/// set_count.set(1);
/// assert_eq!(count.get(), 1);
///
/// // ❌ you could call the getter within the setter
/// // set_count.set(count.get() + 1);
///
/// // ✅ however it's more efficient to use .update() and mutate the value in place
/// set_count.update(|count: &mut i32| *count += 1);
/// assert_eq!(count.get(), 2);
///
/// // ✅ you can create "derived signals" with a Fn() -> T interface
/// let double_count = move || count.get() * 2;
/// set_count.set(0);
/// assert_eq!(double_count(), 0);
/// set_count.set(1);
/// assert_eq!(double_count(), 2);
/// ```
#[inline(always)]
#[track_caller]
pub fn arc_signal<T>(value: T) -> (ArcReadSignal<T>, ArcWriteSignal<T>) {
    ArcRwSignal::new(value).split()
}

/// Creates an arena-allocated signal, the basic reactive primitive.
///
/// A signal is a piece of data that may change over time, and notifies other
/// code when it has changed. This is the atomic unit of reactivity, which begins all other
/// processes of updating.
///
/// Takes the initial value as an argument, and returns a tuple containing a
/// [`ReadSignal`] and a [`WriteSignal`].
///
/// This returns an arena-allocated signal, which is `Copy` and is disposed when its reactive
/// [`Owner`](crate::owner::Owner) cleans up. For a reference-counted signal that lives
/// as long as a reference to it is alive, see [`arc_signal`].
/// ```
/// # use reactive_graph::prelude::*;
/// # use reactive_graph::signal::*; let owner = reactive_graph::owner::Owner::new(); owner.set();
/// let (count, set_count) = signal(0);
///
/// // ✅ calling the getter clones and returns the value
/// //    this can be `count()` on nightly
/// assert_eq!(count.get(), 0);
///
/// // ✅ calling the setter sets the value
/// //    this can be `set_count(1)` on nightly
/// set_count.set(1);
/// assert_eq!(count.get(), 1);
///
/// // ❌ you could call the getter within the setter
/// // set_count.set(count.get() + 1);
///
/// // ✅ however it's more efficient to use .update() and mutate the value in place
/// set_count.update(|count: &mut i32| *count += 1);
/// assert_eq!(count.get(), 2);
///
/// // ✅ you can create "derived signals" with a Fn() -> T interface
/// let double_count = move || count.get() * 2; // signals are `Copy` so you can `move` them anywhere
/// set_count.set(0);
/// assert_eq!(double_count(), 0);
/// set_count.set(1);
/// assert_eq!(double_count(), 2);
/// ```
#[inline(always)]
#[track_caller]
pub fn signal<T: Send + Sync + 'static>(
    value: T,
) -> (ReadSignal<T>, WriteSignal<T>) {
    RwSignal::new(value).split()
}

/// Creates an arena-allocated signal.
///
/// Unlike [`signal`], this does not require the value to be `Send + Sync`. Instead, it is stored
/// on a local arena. Accessing either of the returned signals from another thread will panic.
#[inline(always)]
#[track_caller]
pub fn signal_local<T: 'static>(
    value: T,
) -> (ReadSignal<T, LocalStorage>, WriteSignal<T, LocalStorage>) {
    RwSignal::new_local(value).split()
}

/// Creates an arena-allocated signal, the basic reactive primitive.
///
/// A signal is a piece of data that may change over time, and notifies other
/// code when it has changed. This is the atomic unit of reactivity, which begins all other
/// processes of updating.
///
/// Takes the initial value as an argument, and returns a tuple containing a
/// [`ReadSignal`] and a [`WriteSignal`].
///
/// This returns an arena-allocated signal, which is `Copy` and is disposed when its reactive
/// [`Owner`](crate::owner::Owner) cleans up. For a reference-counted signal that lives
/// as long as a reference to it is alive, see [`arc_signal`].
/// ```
/// # use reactive_graph::prelude::*;
/// # use reactive_graph::signal::*; let owner = reactive_graph::owner::Owner::new(); owner.set();
/// let (count, set_count) = create_signal(0);
///
/// // ✅ calling the getter clones and returns the value
/// //    this can be `count()` on nightly
/// assert_eq!(count.get(), 0);
///
/// // ✅ calling the setter sets the value
/// //    this can be `set_count(1)` on nightly
/// set_count.set(1);
/// assert_eq!(count.get(), 1);
///
/// // ❌ you could call the getter within the setter
/// // set_count.set(count.get() + 1);
///
/// // ✅ however it's more efficient to use .update() and mutate the value in place
/// set_count.update(|count: &mut i32| *count += 1);
/// assert_eq!(count.get(), 2);
///
/// // ✅ you can create "derived signals" with a Fn() -> T interface
/// let double_count = move || count.get() * 2; // signals are `Copy` so you can `move` them anywhere
/// set_count.set(0);
/// assert_eq!(double_count(), 0);
/// set_count.set(1);
/// assert_eq!(double_count(), 2);
/// ```
#[inline(always)]
#[track_caller]
#[deprecated = "This function is being renamed to `signal()` to conform to \
                Rust idioms."]
pub fn create_signal<T: Send + Sync + 'static>(
    value: T,
) -> (ReadSignal<T>, WriteSignal<T>) {
    signal(value)
}

/// Creates a reactive signal with the getter and setter unified in one value.
#[inline(always)]
#[track_caller]
#[deprecated = "This function is being removed to conform to Rust idioms. \
                Please use `RwSignal::new()` instead."]
pub fn create_rw_signal<T: Send + Sync + 'static>(value: T) -> RwSignal<T> {
    RwSignal::new(value)
}

/// A trigger is a data-less signal with the sole purpose of notifying other reactive code of a change.
#[inline(always)]
#[track_caller]
#[deprecated = "This function is being removed to conform to Rust idioms. \
                Please use `ArcTrigger::new()` instead."]
pub fn create_trigger() -> ArcTrigger {
    ArcTrigger::new()
}
