//! Provides a simple trait that unwraps the locks provide by [`std::sync::RwLock`].
//!
//! In every case, this is the same as calling `.expect("lock poisoned")`. However, it
//! does not use `.unwrap()` or `.expect()`, which makes it easier to distinguish from
//! other forms of unwrapping when reading code.
//!
//! ```rust
//! use or_poisoned::OrPoisoned;
//! use std::sync::RwLock;
//!
//! let lock = RwLock::new(String::from("Hello!"));
//!
//! let read = lock.read().or_poisoned();
//! // this is identical to
//! let read = lock.read().unwrap();
//! ```

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use std::sync::{
    LockResult, MutexGuard, PoisonError, RwLockReadGuard, RwLockWriteGuard,
};

/// Unwraps a lock.
pub trait OrPoisoned {
    /// The inner guard type.
    type Inner;

    /// Unwraps the lock.
    ///
    /// ## Panics
    ///
    /// Will panic if the lock is poisoned.
    fn or_poisoned(self) -> Self::Inner;
}

impl<'a, T: ?Sized> OrPoisoned
    for Result<RwLockReadGuard<'a, T>, PoisonError<RwLockReadGuard<'a, T>>>
{
    type Inner = RwLockReadGuard<'a, T>;

    fn or_poisoned(self) -> Self::Inner {
        self.expect("lock poisoned")
    }
}

impl<'a, T: ?Sized> OrPoisoned
    for Result<RwLockWriteGuard<'a, T>, PoisonError<RwLockWriteGuard<'a, T>>>
{
    type Inner = RwLockWriteGuard<'a, T>;

    fn or_poisoned(self) -> Self::Inner {
        self.expect("lock poisoned")
    }
}

impl<'a, T: ?Sized> OrPoisoned for LockResult<MutexGuard<'a, T>> {
    type Inner = MutexGuard<'a, T>;

    fn or_poisoned(self) -> Self::Inner {
        self.expect("lock poisoned")
    }
}
