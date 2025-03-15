//! A value that **might** be wrapped in a [`SendWrapper`] to make non-threadsafe at runtime.

use send_wrapper::SendWrapper;
use std::ops::{Deref, DerefMut};

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
