use crate::{
    traits::{
        DefinedAt, Get, GetUntracked, Read, ReadUntracked, Track, With,
        WithUntracked,
    },
    unwrap_signal,
};
use std::panic::Location;

impl<T> DefinedAt for Option<T>
where
    T: DefinedAt,
{
    fn defined_at(&self) -> Option<&'static Location<'static>> {
        self.as_ref().map(DefinedAt::defined_at).unwrap_or(None)
    }
}

impl<T> Track for Option<T>
where
    T: Track,
{
    fn track(&self) {
        if let Some(signal) = self {
            signal.track();
        }
    }
}

/// An alternative [`ReadUntracked`](crate) trait that works with `Option<Readable>` types.
pub trait ReadUntrackedOptional: Sized + DefinedAt {
    /// The guard type that will be returned, which can be dereferenced to the value.
    type Value;

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

impl<T> ReadUntrackedOptional for Option<T>
where
    Self: DefinedAt,
    T: ReadUntracked,
{
    type Value = Option<<T as ReadUntracked>::Value>;

    fn try_read_untracked(&self) -> Option<Self::Value> {
        Some(if let Some(signal) = self {
            Some(signal.try_read_untracked()?)
        } else {
            None
        })
    }
}

/// An alternative [`Read`](crate) trait that works with `Option<Readable>` types.
pub trait ReadOptional: DefinedAt {
    /// The guard type that will be returned, which can be dereferenced to the value.
    type Value;

    /// Subscribes to the signal, and returns the guard, or `None` if the signal has already been disposed.
    #[track_caller]
    fn try_read(&self) -> Option<Self::Value>;

    /// Subscribes to the signal, and returns the guard.
    ///
    /// # Panics
    /// Panics if you try to access a signal that has been disposed.
    #[track_caller]
    fn read(&self) -> Self::Value {
        self.try_read().unwrap_or_else(unwrap_signal!(self))
    }
}

impl<T> ReadOptional for Option<T>
where
    Self: DefinedAt,
    T: Read,
{
    type Value = Option<<T as Read>::Value>;

    fn try_read(&self) -> Option<Self::Value> {
        Some(if let Some(readable) = self {
            Some(readable.try_read()?)
        } else {
            None
        })
    }
}

/// An alternative [`WithUntracked`](crate) trait that works with `Option<Withable>` types.
pub trait WithUntrackedOptional: DefinedAt {
    /// The type of the value contained in the signal.
    type Value: ?Sized;

    /// Applies the closure to the value, and returns the result,
    /// or `None` if the signal has already been disposed.
    #[track_caller]
    fn try_with_untracked<U>(
        &self,
        fun: impl FnOnce(Option<&Self::Value>) -> U,
    ) -> Option<U>;

    /// Applies the closure to the value, and returns the result.
    ///
    /// # Panics
    /// Panics if you try to access a signal that has been disposed.
    #[track_caller]
    fn with_untracked<U>(
        &self,
        fun: impl FnOnce(Option<&Self::Value>) -> U,
    ) -> U {
        self.try_with_untracked(fun)
            .unwrap_or_else(unwrap_signal!(self))
    }
}

impl<T> WithUntrackedOptional for Option<T>
where
    Self: DefinedAt,
    T: WithUntracked,
    <T as WithUntracked>::Value: Sized,
{
    type Value = <T as WithUntracked>::Value;

    fn try_with_untracked<U>(
        &self,
        fun: impl FnOnce(Option<&Self::Value>) -> U,
    ) -> Option<U> {
        if let Some(signal) = self {
            Some(signal.try_with_untracked(|val| fun(Some(val)))?)
        } else {
            Some(fun(None))
        }
    }
}

/// An alternative [`With`](crate) trait that works with `Option<Withable>` types.
pub trait WithOptional: DefinedAt {
    /// The type of the value contained in the signal.
    type Value: ?Sized;

    /// Subscribes to the signal, applies the closure to the value, and returns the result,
    /// or `None` if the signal has already been disposed.
    #[track_caller]
    fn try_with<U>(
        &self,
        fun: impl FnOnce(Option<&Self::Value>) -> U,
    ) -> Option<U>;

    /// Subscribes to the signal, applies the closure to the value, and returns the result.
    ///
    /// # Panics
    /// Panics if you try to access a signal that has been disposed.
    #[track_caller]
    fn with<U>(&self, fun: impl FnOnce(Option<&Self::Value>) -> U) -> U {
        self.try_with(fun).unwrap_or_else(unwrap_signal!(self))
    }
}

impl<T> WithOptional for Option<T>
where
    Self: DefinedAt,
    T: With,
    <T as With>::Value: Sized,
{
    type Value = <T as With>::Value;

    fn try_with<U>(
        &self,
        fun: impl FnOnce(Option<&Self::Value>) -> U,
    ) -> Option<U> {
        if let Some(signal) = self {
            Some(signal.try_with(|val| fun(Some(val)))?)
        } else {
            Some(fun(None))
        }
    }
}

impl<T> GetUntracked for Option<T>
where
    Self: DefinedAt,
    T: GetUntracked,
{
    type Value = Option<<T as GetUntracked>::Value>;

    fn try_get_untracked(&self) -> Option<Self::Value> {
        Some(if let Some(signal) = self {
            Some(signal.try_get_untracked()?)
        } else {
            None
        })
    }
}

impl<T> Get for Option<T>
where
    Self: DefinedAt,
    T: Get,
{
    type Value = Option<<T as Get>::Value>;

    fn try_get(&self) -> Option<Self::Value> {
        Some(if let Some(signal) = self {
            Some(signal.try_get()?)
        } else {
            None
        })
    }
}

/// Helper trait to implement flatten() on `Option<&Option<T>>`.
pub trait FlattenOptionRefOption {
    /// The type of the value contained in the double option.
    type Value;

    /// Converts from `Option<&Option<T>>` to `Option<&T>`.
    fn flatten(&self) -> Option<&Self::Value>;
}

impl<'a, T> FlattenOptionRefOption for Option<&'a Option<T>> {
    type Value = T;

    fn flatten(&self) -> Option<&'a T> {
        self.map(Option::as_ref).flatten()
    }
}
