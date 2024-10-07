//! A series of traits to implement the behavior of reactive primitive, especially signals.
//!
//! ## Principles
//! 1. **Composition**: Most of the traits are implemented as combinations of more primitive base traits,
//!    and blanket implemented for all types that implement those traits.
//! 2. **Fallibility**: Most traits includes a `try_` variant, which returns `None` if the method
//!    fails (e.g., if signals are arena allocated and this can't be found, or if an `RwLock` is
//!    poisoned).
//!
//! ## Metadata Traits
//! - [`DefinedAt`] is used for debugging in the case of errors and should be implemented for all
//!   signal types.
//! - [`IsDisposed`] checks whether a signal is currently accessible.
//!
//! ## Base Traits
//! | Trait             | Mode  | Description                                                                           |
//! |-------------------|-------|---------------------------------------------------------------------------------------|
//! | [`Track`]         | —     | Tracks changes to this value, adding it as a source of the current reactive observer. |
//! | [`Trigger`]       | —     | Notifies subscribers that this value has changed.                                     |
//! | [`ReadUntracked`] | Guard | Gives immutable access to the value of this signal.                                   |
//! | [`Write`]     | Guard | Gives mutable access to the value of this signal.
//!
//! ## Derived Traits
//!
//! ### Access
//! | Trait             | Mode          | Composition                   | Description
//! |-------------------|---------------|-------------------------------|------------
//! | [`WithUntracked`] | `fn(&T) -> U` | [`ReadUntracked`]                  | Applies closure to the current value of the signal and returns result.
//! | [`With`]          | `fn(&T) -> U` | [`ReadUntracked`] + [`Track`]      | Applies closure to the current value of the signal and returns result, with reactive tracking.
//! | [`GetUntracked`]  | `T`           | [`WithUntracked`] + [`Clone`] | Clones the current value of the signal.
//! | [`Get`]           | `T`           | [`GetUntracked`] + [`Track`]  | Clones the current value of the signal, with reactive tracking.
//!
//! ### Update
//! | Trait               | Mode          | Composition                       | Description
//! |---------------------|---------------|-----------------------------------|------------
//! | [`UpdateUntracked`] | `fn(&mut T)`  | [`Write`]                     | Applies closure to the current value to update it, but doesn't notify subscribers.
//! | [`Update`]          | `fn(&mut T)`  | [`UpdateUntracked`] + [`Trigger`] | Applies closure to the current value to update it, and notifies subscribers.
//! | [`Set`]             | `T`           | [`Update`]                        | Sets the value to a new value, and notifies subscribers.
//!
//! ## Using the Traits
//!
//! These traits are designed so that you can implement as few as possible, and the rest will be
//! implemented automatically.
//!
//! For example, if you have a struct for which you can implement [`ReadUntracked`] and [`Track`], then
//! [`WithUntracked`] and [`With`] will be implemented automatically (as will [`GetUntracked`] and
//! [`Get`] for `Clone` types). But if you cannot implement [`ReadUntracked`] (because, for example,
//! there isn't an `RwLock` so you can't wrap in a [`ReadGuard`](crate::signal::guards::ReadGuard),
//! but you can still implement [`WithUntracked`] and [`Track`], the same traits will still be implemented.

use crate::{
    effect::Effect,
    graph::{Observer, Source, Subscriber, ToAnySource},
    owner::Owner,
    signal::{arc_signal, guards::UntrackedWriteGuard, ArcReadSignal},
};
use any_spawner::Executor;
use futures::{Stream, StreamExt};
use std::{
    ops::{Deref, DerefMut},
    panic::Location,
};

#[doc(hidden)]
/// Provides a sensible panic message for accessing disposed signals.
#[macro_export]
macro_rules! unwrap_signal {
    ($signal:ident) => {{
        #[cfg(debug_assertions)]
        let location = std::panic::Location::caller();
        || {
            #[cfg(debug_assertions)]
            {
                panic!(
                    "{}",
                    $crate::traits::panic_getting_disposed_signal(
                        $signal.defined_at(),
                        location
                    )
                );
            }
            #[cfg(not(debug_assertions))]
            {
                panic!(
                    "Tried to access a reactive value that has already been \
                     disposed."
                );
            }
        }
    }};
}

/// Allows disposing an arena-allocated signal before its owner has been disposed.
pub trait Dispose {
    /// Disposes of the signal. This:
    /// 1. Detaches the signal from the reactive graph, preventing it from triggering
    ///    further updates; and
    /// 2. Drops the value contained in the signal.
    fn dispose(self);
}

/// Allows tracking the value of some reactive data.
pub trait Track {
    /// Subscribes to this signal in the current reactive scope without doing anything with its value.
    fn track(&self);
}

impl<T: Source + ToAnySource + DefinedAt> Track for T {
    #[track_caller]
    fn track(&self) {
        if self.is_disposed() {
            return;
        }

        if let Some(subscriber) = Observer::get() {
            subscriber.add_source(self.to_any_source());
            self.add_subscriber(subscriber);
        } else {
            #[cfg(all(debug_assertions, feature = "effects"))]
            {
                use crate::diagnostics::SpecialNonReactiveZone;

                if !SpecialNonReactiveZone::is_inside() {
                    let called_at = Location::caller();
                    let ty = std::any::type_name::<T>();
                    let defined_at = self
                        .defined_at()
                        .map(ToString::to_string)
                        .unwrap_or_else(|| String::from("{unknown}"));
                    crate::log_warning(format_args!(
                        "At {called_at}, you access a {ty} (defined at \
                         {defined_at}) outside a reactive tracking context. \
                         This might mean your app is not responding to \
                         changes in signal values in the way you \
                         expect.\n\nHere’s how to fix it:\n\n1. If this is \
                         inside a `view!` macro, make sure you are passing a \
                         function, not a value.\n  ❌ NO  <p>{{x.get() * \
                         2}}</p>\n  ✅ YES <p>{{move || x.get() * \
                         2}}</p>\n\n2. If it’s in the body of a component, \
                         try wrapping this access in a closure: \n  ❌ NO  \
                         let y = x.get() * 2\n  ✅ YES let y = move || \
                         x.get() * 2.\n\n3. If you’re *trying* to access the \
                         value without tracking, use `.get_untracked()` or \
                         `.with_untracked()` instead."
                    ));
                }
            }
        }
    }
}

/// Give read-only access to a signal's value by reference through a guard type,
/// without tracking the value reactively.
pub trait ReadUntracked: Sized + DefinedAt {
    /// The guard type that will be returned, which can be dereferenced to the value.
    type Value: Deref;

    /// Returns the guard, or `None` if the signal has already been disposed.
    #[track_caller]
    fn try_read_untracked(&self) -> Option<Self::Value>;

    /// Returns the guard.
    ///
    /// # Panics
    /// Panics if you try to access a signal that has been disposed.
    #[track_caller]
    fn read_untracked(&self) -> Self::Value {
        self.try_read_untracked()
            .unwrap_or_else(unwrap_signal!(self))
    }
}

/// Give read-only access to a signal's value by reference through a guard type,
/// and subscribes the active reactive observer (an effect or computed) to changes in its value.
pub trait Read {
    /// The guard type that will be returned, which can be dereferenced to the value.
    type Value: Deref;

    /// Subscribes to the signal, and returns the guard, or `None` if the signal has already been disposed.
    #[track_caller]
    fn try_read(&self) -> Option<Self::Value>;

    /// Subscribes to the signal, and returns the guard.
    ///
    /// # Panics
    /// Panics if you try to access a signal that has been disposed.
    #[track_caller]
    fn read(&self) -> Self::Value;
}

impl<T> Read for T
where
    T: Track + ReadUntracked,
{
    type Value = T::Value;

    fn try_read(&self) -> Option<Self::Value> {
        self.track();
        self.try_read_untracked()
    }

    fn read(&self) -> Self::Value {
        self.track();
        self.read_untracked()
    }
}

/// A reactive, mutable guard that can be untracked to prevent it from notifying subscribers when
/// it is dropped.
pub trait UntrackableGuard: DerefMut {
    /// Removes the notifier from the guard, such that it will no longer notify subscribers when it is dropped.
    fn untrack(&mut self);
}

/// Gives mutable access to a signal's value through a guard type. When the guard is dropped, the
/// signal's subscribers will be notified.
pub trait Write: Sized + DefinedAt + Notify {
    /// The type of the signal's value.
    type Value: Sized + 'static;

    /// Returns the guard, or `None` if the signal has already been disposed.
    fn try_write(&self) -> Option<impl UntrackableGuard<Target = Self::Value>>;

    // Returns a guard that will not notify subscribers when dropped,
    /// or `None` if the signal has already been disposed.
    fn try_write_untracked(
        &self,
    ) -> Option<impl DerefMut<Target = Self::Value>>;

    /// Returns the guard.
    ///
    /// # Panics
    /// Panics if you try to access a signal that has been disposed.
    fn write(&self) -> impl UntrackableGuard<Target = Self::Value> {
        self.try_write().unwrap_or_else(unwrap_signal!(self))
    }

    /// Returns a guard that will not notify subscribers when dropped.
    ///
    /// # Panics
    /// Panics if you try to access a signal that has been disposed.
    fn write_untracked(&self) -> impl DerefMut<Target = Self::Value> {
        self.try_write_untracked()
            .unwrap_or_else(unwrap_signal!(self))
    }
}

/// Give read-only access to a signal's value by reference inside a closure,
/// without tracking the value reactively.
pub trait WithUntracked: DefinedAt {
    /// The type of the value contained in the signal.
    type Value: ?Sized;

    /// Applies the closure to the value, and returns the result,
    /// or `None` if the signal has already been disposed.
    #[track_caller]
    fn try_with_untracked<U>(
        &self,
        fun: impl FnOnce(&Self::Value) -> U,
    ) -> Option<U>;

    /// Applies the closure to the value, and returns the result.
    ///
    /// # Panics
    /// Panics if you try to access a signal that has been disposed.
    #[track_caller]
    fn with_untracked<U>(&self, fun: impl FnOnce(&Self::Value) -> U) -> U {
        self.try_with_untracked(fun)
            .unwrap_or_else(unwrap_signal!(self))
    }
}

impl<T> WithUntracked for T
where
    T: DefinedAt + ReadUntracked,
{
    type Value = <<Self as ReadUntracked>::Value as Deref>::Target;

    fn try_with_untracked<U>(
        &self,
        fun: impl FnOnce(&Self::Value) -> U,
    ) -> Option<U> {
        self.try_read_untracked().map(|value| fun(&value))
    }
}

/// Give read-only access to a signal's value by reference inside a closure,
/// and subscribes the active reactive observer (an effect or computed) to changes in its value.
pub trait With: DefinedAt {
    /// The type of the value contained in the signal.
    type Value: ?Sized;

    /// Subscribes to the signal, applies the closure to the value, and returns the result,
    /// or `None` if the signal has already been disposed.
    #[track_caller]
    fn try_with<U>(&self, fun: impl FnOnce(&Self::Value) -> U) -> Option<U>;

    /// Subscribes to the signal, applies the closure to the value, and returns the result.
    ///
    /// # Panics
    /// Panics if you try to access a signal that has been disposed.
    #[track_caller]
    fn with<U>(&self, fun: impl FnOnce(&Self::Value) -> U) -> U {
        self.try_with(fun).unwrap_or_else(unwrap_signal!(self))
    }
}

impl<T> With for T
where
    T: WithUntracked + Track,
{
    type Value = <T as WithUntracked>::Value;

    #[track_caller]
    fn try_with<U>(&self, fun: impl FnOnce(&Self::Value) -> U) -> Option<U> {
        self.track();
        self.try_with_untracked(fun)
    }
}

/// Clones the value of the signal, without tracking the value reactively.
pub trait GetUntracked: DefinedAt {
    /// The type of the value contained in the signal.
    type Value;

    /// Clones and returns the value of the signal,
    /// or `None` if the signal has already been disposed.
    #[track_caller]
    fn try_get_untracked(&self) -> Option<Self::Value>;

    /// Clones and returns the value of the signal,
    ///
    /// # Panics
    /// Panics if you try to access a signal that has been disposed.
    #[track_caller]
    fn get_untracked(&self) -> Self::Value {
        self.try_get_untracked()
            .unwrap_or_else(unwrap_signal!(self))
    }
}

impl<T> GetUntracked for T
where
    T: WithUntracked,
    T::Value: Clone,
{
    type Value = <Self as WithUntracked>::Value;

    fn try_get_untracked(&self) -> Option<Self::Value> {
        self.try_with_untracked(Self::Value::clone)
    }
}

/// Clones the value of the signal, without tracking the value reactively.
/// and subscribes the active reactive observer (an effect or computed) to changes in its value.
pub trait Get: DefinedAt {
    /// The type of the value contained in the signal.
    type Value: Clone;

    /// Subscribes to the signal, then clones and returns the value of the signal,
    /// or `None` if the signal has already been disposed.
    #[track_caller]
    fn try_get(&self) -> Option<Self::Value>;

    /// Subscribes to the signal, then clones and returns the value of the signal.
    ///
    /// # Panics
    /// Panics if you try to access a signal that has been disposed.
    #[track_caller]
    fn get(&self) -> Self::Value {
        self.try_get().unwrap_or_else(unwrap_signal!(self))
    }
}

impl<T> Get for T
where
    T: With,
    T::Value: Clone,
{
    type Value = <T as With>::Value;

    #[track_caller]
    fn try_get(&self) -> Option<Self::Value> {
        self.try_with(Self::Value::clone)
    }
}

/// Notifies subscribers of a change in this signal.
pub trait Notify {
    /// Notifies subscribers of a change in this signal.
    fn notify(&self);
}

/// Updates the value of a signal by applying a function that updates it in place,
/// without notifying subscribers.
pub trait UpdateUntracked: DefinedAt {
    /// The type of the value contained in the signal.
    type Value;

    /// Updates the value by applying a function, returning the value returned by that function.
    /// Does not notify subscribers that the signal has changed.
    ///
    /// # Panics
    /// Panics if you try to update a signal that has been disposed.
    #[track_caller]
    fn update_untracked<U>(
        &self,
        fun: impl FnOnce(&mut Self::Value) -> U,
    ) -> U {
        self.try_update_untracked(fun)
            .unwrap_or_else(unwrap_signal!(self))
    }

    /// Updates the value by applying a function, returning the value returned by that function,
    /// or `None` if the signal has already been disposed.
    /// Does not notify subscribers that the signal has changed.
    fn try_update_untracked<U>(
        &self,
        fun: impl FnOnce(&mut Self::Value) -> U,
    ) -> Option<U>;
}

impl<T> UpdateUntracked for T
where
    T: Write,
{
    type Value = <Self as Write>::Value;

    #[track_caller]
    fn try_update_untracked<U>(
        &self,
        fun: impl FnOnce(&mut Self::Value) -> U,
    ) -> Option<U> {
        let mut guard = self.try_write_untracked()?;
        Some(fun(&mut *guard))
    }
}

/// Updates the value of a signal by applying a function that updates it in place,
/// notifying its subscribers that the value has changed.
pub trait Update {
    /// The type of the value contained in the signal.
    type Value;

    /// Updates the value of the signal and notifies subscribers.
    #[track_caller]
    fn update(&self, fun: impl FnOnce(&mut Self::Value)) {
        self.try_update(fun);
    }

    /// Updates the value of the signal, but only notifies subscribers if the function
    /// returns `true`.
    #[track_caller]
    fn maybe_update(&self, fun: impl FnOnce(&mut Self::Value) -> bool) {
        self.try_maybe_update(|val| {
            let did_update = fun(val);
            (did_update, ())
        });
    }

    /// Updates the value of the signal and notifies subscribers, returning the value that is
    /// returned by the update function, or `None` if the signal has already been disposed.
    #[track_caller]
    fn try_update<U>(
        &self,
        fun: impl FnOnce(&mut Self::Value) -> U,
    ) -> Option<U> {
        self.try_maybe_update(|val| (true, fun(val)))
    }

    /// Updates the value of the signal, notifying subscribers if the update function returns
    /// `(true, _)`, and returns the value returned by the update function,
    /// or `None` if the signal has already been disposed.
    fn try_maybe_update<U>(
        &self,
        fun: impl FnOnce(&mut Self::Value) -> (bool, U),
    ) -> Option<U>;
}

impl<T> Update for T
where
    T: Write,
{
    type Value = <Self as Write>::Value;

    #[track_caller]
    fn try_maybe_update<U>(
        &self,
        fun: impl FnOnce(&mut Self::Value) -> (bool, U),
    ) -> Option<U> {
        let mut lock = self.try_write()?;
        let (did_update, val) = fun(&mut *lock);
        if !did_update {
            lock.untrack();
        }
        drop(lock);
        Some(val)
    }
}

/// Updates the value of the signal by replacing it.
pub trait Set {
    /// The type of the value contained in the signal.
    type Value;

    /// Updates the value by replacing it, and notifies subscribers that it has changed.
    fn set(&self, value: Self::Value);

    /// Updates the value by replacing it, and notifies subscribers that it has changed.
    ///
    /// If the signal has already been disposed, returns `Some(value)` with the value that was
    /// passed in. Otherwise, returns `None`.
    fn try_set(&self, value: Self::Value) -> Option<Self::Value>;
}

impl<T> Set for T
where
    T: Update + IsDisposed,
{
    type Value = <Self as Update>::Value;

    #[track_caller]
    fn set(&self, value: Self::Value) {
        self.try_update(|n| *n = value);
    }

    #[track_caller]
    fn try_set(&self, value: Self::Value) -> Option<Self::Value> {
        if self.is_disposed() {
            Some(value)
        } else {
            self.set(value);
            None
        }
    }
}

/// Allows converting a signal into an async [`Stream`].
pub trait ToStream<T> {
    /// Generates a [`Stream`] that emits the new value of the signal
    /// whenever it changes.
    ///
    /// # Panics
    /// Panics if you try to access a signal that is owned by a reactive node that has been disposed.
    #[track_caller]
    fn to_stream(&self) -> impl Stream<Item = T> + Send;
}

impl<S> ToStream<S::Value> for S
where
    S: Clone + Get + Send + Sync + 'static,
    S::Value: Send + 'static,
{
    fn to_stream(&self) -> impl Stream<Item = S::Value> + Send {
        let (tx, rx) = futures::channel::mpsc::unbounded();

        let close_channel = tx.clone();

        Owner::on_cleanup(move || close_channel.close_channel());

        Effect::new_isomorphic({
            let this = self.clone();
            move |_| {
                let _ = tx.unbounded_send(this.get());
            }
        });

        rx
    }
}

/// Allows creating a signal from an async [`Stream`].
pub trait FromStream<T> {
    /// Creates a signal that contains the latest value of the stream.
    #[track_caller]
    fn from_stream(stream: impl Stream<Item = T> + Send + 'static) -> Self;

    /// Creates a signal that contains the latest value of the stream.
    #[track_caller]
    fn from_stream_unsync(stream: impl Stream<Item = T> + 'static) -> Self;
}

impl<S, T> FromStream<T> for S
where
    S: From<ArcReadSignal<Option<T>>> + Send + Sync,
    T: Send + Sync + 'static,
{
    fn from_stream(stream: impl Stream<Item = T> + Send + 'static) -> Self {
        let (read, write) = arc_signal(None);
        let mut stream = Box::pin(stream);
        crate::spawn(async move {
            while let Some(value) = stream.next().await {
                write.set(Some(value));
            }
        });
        read.into()
    }

    fn from_stream_unsync(stream: impl Stream<Item = T> + 'static) -> Self {
        let (read, write) = arc_signal(None);
        let mut stream = Box::pin(stream);
        Executor::spawn_local(async move {
            while let Some(value) = stream.next().await {
                write.set(Some(value));
            }
        });
        read.into()
    }
}

/// Checks whether a signal has already been disposed.
pub trait IsDisposed {
    /// If `true`, the signal cannot be accessed without a panic.
    fn is_disposed(&self) -> bool;
}

/// Describes where the signal was defined. This is used for diagnostic warnings and is purely a
/// debug-mode tool.
pub trait DefinedAt {
    /// Returns the location at which the signal was defined. This is usually simply `None` in
    /// release mode.
    fn defined_at(&self) -> Option<&'static Location<'static>>;
}

#[doc(hidden)]
pub fn panic_getting_disposed_signal(
    defined_at: Option<&'static Location<'static>>,
    location: &'static Location<'static>,
) -> String {
    if let Some(defined_at) = defined_at {
        format!(
            "At {location}, you tried to access a reactive value which was \
             defined at {defined_at}, but it has already been disposed."
        )
    } else {
        format!(
            "At {location}, you tried to access a reactive value, but it has \
             already been disposed."
        )
    }
}

/// A variation of the [`Read`] trait that provides a signposted "always-non-reactive" API.
/// E.g. for [`StoredValue`](`crate::owner::StoredValue`).
pub trait ReadValue: Sized + DefinedAt {
    /// The guard type that will be returned, which can be dereferenced to the value.
    type Value: Deref;

    /// Returns the non-reactive guard, or `None` if the value has already been disposed.
    #[track_caller]
    fn try_read_value(&self) -> Option<Self::Value>;

    /// Returns the non-reactive guard.
    ///
    /// # Panics
    /// Panics if you try to access a value that has been disposed.
    #[track_caller]
    fn read_value(&self) -> Self::Value {
        self.try_read_value().unwrap_or_else(unwrap_signal!(self))
    }
}

/// A variation of the [`With`] trait that provides a signposted "always-non-reactive" API.
/// E.g. for [`StoredValue`](`crate::owner::StoredValue`).
pub trait WithValue: DefinedAt {
    /// The type of the value contained in the value.
    type Value: ?Sized;

    /// Applies the closure to the value, non-reactively, and returns the result,
    /// or `None` if the value has already been disposed.
    #[track_caller]
    fn try_with_value<U>(
        &self,
        fun: impl FnOnce(&Self::Value) -> U,
    ) -> Option<U>;

    /// Applies the closure to the value, non-reactively, and returns the result.
    ///
    /// # Panics
    /// Panics if you try to access a value that has been disposed.
    #[track_caller]
    fn with_value<U>(&self, fun: impl FnOnce(&Self::Value) -> U) -> U {
        self.try_with_value(fun)
            .unwrap_or_else(unwrap_signal!(self))
    }
}

impl<T> WithValue for T
where
    T: DefinedAt + ReadValue,
{
    type Value = <<Self as ReadValue>::Value as Deref>::Target;

    fn try_with_value<U>(
        &self,
        fun: impl FnOnce(&Self::Value) -> U,
    ) -> Option<U> {
        self.try_read_value().map(|value| fun(&value))
    }
}

/// A variation of the [`Get`] trait that provides a signposted "always-non-reactive" API.
/// E.g. for [`StoredValue`](`crate::owner::StoredValue`).
pub trait GetValue: DefinedAt {
    /// The type of the value contained in the value.
    type Value: Clone;

    /// Clones and returns the value of the value, non-reactively,
    /// or `None` if the value has already been disposed.
    #[track_caller]
    fn try_get_value(&self) -> Option<Self::Value>;

    /// Clones and returns the value of the value, non-reactively.
    ///
    /// # Panics
    /// Panics if you try to access a value that has been disposed.
    #[track_caller]
    fn get_value(&self) -> Self::Value {
        self.try_get_value().unwrap_or_else(unwrap_signal!(self))
    }
}

impl<T> GetValue for T
where
    T: WithValue,
    T::Value: Clone,
{
    type Value = <Self as WithValue>::Value;

    fn try_get_value(&self) -> Option<Self::Value> {
        self.try_with_value(Self::Value::clone)
    }
}

/// A variation of the [`Write`] trait that provides a signposted "always-non-reactive" API.
/// E.g. for [`StoredValue`](`crate::owner::StoredValue`).
pub trait WriteValue: Sized + DefinedAt {
    /// The type of the value's value.
    type Value: Sized + 'static;

    /// Returns a non-reactive write guard, or `None` if the value has already been disposed.
    #[track_caller]
    fn try_write_value(&self) -> Option<UntrackedWriteGuard<Self::Value>>;

    /// Returns a non-reactive write guard.
    ///
    /// # Panics
    /// Panics if you try to access a value that has been disposed.
    #[track_caller]
    fn write_value(&self) -> UntrackedWriteGuard<Self::Value> {
        self.try_write_value().unwrap_or_else(unwrap_signal!(self))
    }
}

/// A variation of the [`Update`] trait that provides a signposted "always-non-reactive" API.
/// E.g. for [`StoredValue`](`crate::owner::StoredValue`).
pub trait UpdateValue: DefinedAt {
    /// The type of the value contained in the value.
    type Value;

    /// Updates the value, returning the value that is
    /// returned by the update function, or `None` if the value has already been disposed.
    #[track_caller]
    fn try_update_value<U>(
        &self,
        fun: impl FnOnce(&mut Self::Value) -> U,
    ) -> Option<U>;

    /// Updates the value.
    #[track_caller]
    fn update_value(&self, fun: impl FnOnce(&mut Self::Value)) {
        self.try_update_value(fun);
    }
}

impl<T> UpdateValue for T
where
    T: WriteValue,
{
    type Value = <Self as WriteValue>::Value;

    #[track_caller]
    fn try_update_value<U>(
        &self,
        fun: impl FnOnce(&mut Self::Value) -> U,
    ) -> Option<U> {
        let mut guard = self.try_write_value()?;
        Some(fun(&mut *guard))
    }
}

/// A variation of the [`Set`] trait that provides a signposted "always-non-reactive" API.
/// E.g. for [`StoredValue`](`crate::owner::StoredValue`).
pub trait SetValue: DefinedAt {
    /// The type of the value contained in the value.
    type Value;

    /// Updates the value by replacing it, non-reactively.
    ///
    /// If the value has already been disposed, returns `Some(value)` with the value that was
    /// passed in. Otherwise, returns `None`.
    #[track_caller]
    fn try_set_value(&self, value: Self::Value) -> Option<Self::Value>;

    /// Updates the value by replacing it, non-reactively.
    #[track_caller]
    fn set_value(&self, value: Self::Value) {
        self.try_set_value(value);
    }
}

impl<T> SetValue for T
where
    T: WriteValue,
{
    type Value = <Self as WriteValue>::Value;

    fn try_set_value(&self, value: Self::Value) -> Option<Self::Value> {
        // Unlike most other traits, for these None actually means success:
        if let Some(mut guard) = self.try_write_value() {
            *guard = value;
            None
        } else {
            Some(value)
        }
    }
}
