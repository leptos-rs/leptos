//! A value that **might** be wrapped in a [`SendWrapper`] to make non-threadsafe at runtime.

use send_wrapper::SendWrapper;
use std::{
    fmt::{Debug, Formatter},
    ops::{Deref, DerefMut},
};

/// A value that might be wrapped in a [`SendWrapper`] to make non-threadsafe at runtime.
pub enum MaybeSendWrapper<T> {
    /// A threadsafe value.
    Threadsafe(T),
    /// A non-threadsafe value. If accessed from a different thread, it will panic.
    Local(SendWrapper<T>),
}

impl<T> MaybeSendWrapper<T> {
    /// Map from one wrapped value to another.
    ///
    /// # Panics
    /// Panics if the [`MaybeSendWrapper::Local`] variant and it is called from a different thread than the one the instance has been created with.    
    pub fn map<Out>(self, f: impl FnOnce(T) -> Out) -> MaybeSendWrapper<Out> {
        match self {
            Self::Threadsafe(value) => MaybeSendWrapper::Threadsafe(f(value)),
            Self::Local(value) => {
                MaybeSendWrapper::Local(SendWrapper::new(f(value.take())))
            }
        }
    }

    /// Consume the value.
    ///
    /// # Panics
    /// Panics if the Local() variant and it is called from a different thread than the one the instance has been created with.
    pub fn take(self) -> T {
        match self {
            Self::Threadsafe(value) => value,
            Self::Local(value) => value.take(),
        }
    }
}

impl<T> Deref for MaybeSendWrapper<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        match self {
            Self::Threadsafe(value) => value,
            Self::Local(value) => value.deref(),
        }
    }
}

impl<T> DerefMut for MaybeSendWrapper<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            Self::Threadsafe(value) => value,
            Self::Local(value) => value.deref_mut(),
        }
    }
}

impl<T: Debug> Debug for MaybeSendWrapper<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Threadsafe(value) => {
                write!(f, "MaybeSendWrapper::Threadsafe({:?})", value)
            }
            Self::Local(value) => {
                write!(f, "MaybeSendWrapper::Local({:?})", value)
            }
        }
    }
}

impl<T: Clone> Clone for MaybeSendWrapper<T> {
    fn clone(&self) -> Self {
        match self {
            Self::Threadsafe(value) => Self::Threadsafe(value.clone()),
            Self::Local(value) => Self::Local(value.clone()),
        }
    }
}
