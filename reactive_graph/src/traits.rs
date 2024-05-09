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
//! | [`Writeable`]     | Guard | Gives mutable access to the value of this signal.
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
//! | [`UpdateUntracked`] | `fn(&mut T)`  | [`Writeable`]                     | Applies closure to the current value to update it, but doesn't notify subscribers.
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
//! there isn't an `RwLock` you can wrap in a [`SignalReadGuard`](crate::signal::SignalReadGuard),
//! but you can still implement [`WithUntracked`] and [`Track`], the same traits will still be implemented.

use crate::{
    graph::{Observer, Source, Subscriber, ToAnySource},
    signal::guards::{UntrackedWriteGuard, WriteGuard},
};
use std::{ops::Deref, panic::Location};

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

pub trait Dispose {
    fn dispose(self);
}

pub trait Track {
    fn track(&self);
}

impl<T: Source + ToAnySource> Track for T {
    #[track_caller]
    fn track(&self) {
        if let Some(subscriber) = Observer::get() {
            subscriber.add_source(self.to_any_source());
            self.add_subscriber(subscriber);
        }
    }
}

pub trait ReadUntracked: Sized + DefinedAt {
    type Value: Deref;

    #[track_caller]
    fn try_read_untracked(&self) -> Option<Self::Value>;

    #[track_caller]
    fn read_untracked(&self) -> Self::Value {
        self.try_read_untracked()
            .unwrap_or_else(unwrap_signal!(self))
    }
}

pub trait Read {
    type Value: Deref;

    #[track_caller]
    fn try_read(&self) -> Option<Self::Value>;

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

pub trait Writeable: Sized + DefinedAt + Trigger {
    type Value: Sized;

    fn try_write(&self) -> Option<WriteGuard<'_, Self, Self::Value>>;

    fn try_write_untracked(
        &self,
    ) -> Option<UntrackedWriteGuard<'_, Self::Value>>;

    fn write(&self) -> WriteGuard<'_, Self, Self::Value> {
        self.try_write().unwrap_or_else(unwrap_signal!(self))
    }

    fn write_untracked(&self) -> UntrackedWriteGuard<'_, Self::Value> {
        self.try_write_untracked()
            .unwrap_or_else(unwrap_signal!(self))
    }
}

pub trait WithUntracked: DefinedAt {
    type Value: ?Sized;

    #[track_caller]
    fn try_with_untracked<U>(
        &self,
        fun: impl FnOnce(&Self::Value) -> U,
    ) -> Option<U>;

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

pub trait With: DefinedAt {
    type Value: ?Sized;

    fn try_with<U>(&self, fun: impl FnOnce(&Self::Value) -> U) -> Option<U>;

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

pub trait GetUntracked: DefinedAt {
    type Value;

    fn try_get_untracked(&self) -> Option<Self::Value>;

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

pub trait Get: DefinedAt {
    type Value: Clone;

    fn try_get(&self) -> Option<Self::Value>;

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

    fn try_get(&self) -> Option<Self::Value> {
        self.try_with(Self::Value::clone)
    }
}

pub trait Trigger {
    fn trigger(&self);
}

pub trait UpdateUntracked: DefinedAt {
    type Value;

    fn update_untracked<U>(
        &self,
        fun: impl FnOnce(&mut Self::Value) -> U,
    ) -> U {
        self.try_update_untracked(fun)
            .unwrap_or_else(unwrap_signal!(self))
    }

    fn try_update_untracked<U>(
        &self,
        fun: impl FnOnce(&mut Self::Value) -> U,
    ) -> Option<U>;
}

impl<T> UpdateUntracked for T
where
    T: Writeable,
{
    type Value = <Self as Writeable>::Value;

    fn try_update_untracked<U>(
        &self,
        fun: impl FnOnce(&mut Self::Value) -> U,
    ) -> Option<U> {
        let mut guard = self.try_write_untracked()?;
        Some(fun(&mut *guard))
    }
}

pub trait Update {
    type Value;

    fn update(&self, fun: impl FnOnce(&mut Self::Value)) {
        self.try_update(fun);
    }

    fn maybe_update(&self, fun: impl FnOnce(&mut Self::Value) -> bool) {
        self.try_maybe_update(|val| {
            let did_update = fun(val);
            (did_update, ())
        });
    }

    fn try_update<U>(
        &self,
        fun: impl FnOnce(&mut Self::Value) -> U,
    ) -> Option<U> {
        self.try_maybe_update(|val| (true, fun(val)))
    }

    fn try_maybe_update<U>(
        &self,
        fun: impl FnOnce(&mut Self::Value) -> (bool, U),
    ) -> Option<U>;
}

impl<T> Update for T
where
    T: UpdateUntracked + Trigger,
{
    type Value = <Self as UpdateUntracked>::Value;

    fn try_maybe_update<U>(
        &self,
        fun: impl FnOnce(&mut Self::Value) -> (bool, U),
    ) -> Option<U> {
        let (did_update, val) = self.try_update_untracked(fun)?;
        if did_update {
            self.trigger();
        }
        Some(val)
    }
}

pub trait Set: Update + IsDisposed {
    fn set(&self, value: impl Into<Self::Value>) {
        self.update(|n| *n = value.into());
    }

    fn try_set(&self, value: impl Into<Self::Value>) -> Option<Self::Value> {
        if self.is_disposed() {
            Some(value.into())
        } else {
            self.set(value);
            None
        }
    }
}

impl<T> Set for T where T: Update + IsDisposed {}

pub trait IsDisposed {
    fn is_disposed(&self) -> bool;
}

pub trait DefinedAt {
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
