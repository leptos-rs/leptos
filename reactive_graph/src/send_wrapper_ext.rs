//! Additional wrapper utilities for [`send_wrapper::SendWrapper`].

use send_wrapper::SendWrapper;
use std::{
    fmt::{Debug, Formatter},
    ops::{Deref, DerefMut},
};

/// An optional value that might be wrapped in [`SendWrapper`].
///
/// This struct is useful because:
/// - Can be dereffed to &Option<T>, even when T is wrapped in a SendWrapper.
/// - Until [`DerefMut`] is called, the None case will not construct a SendWrapper, so no panics if initialised when None and dropped on a different thread. Any access other than [`DerefMut`] will not construct a SendWrapper.
pub struct MaybeSendWrapperOption<T> {
    inner: Inner<T>,
}

unsafe impl<T> Send for MaybeSendWrapperOption<T> {}
unsafe impl<T> Sync for MaybeSendWrapperOption<T> {}

enum Inner<T> {
    /// A threadsafe value.
    Threadsafe(Option<T>),
    /// A non-threadsafe value. If accessed/dropped from a different thread in the Some() variant, it will panic.
    Local(Option<SendWrapper<Option<T>>>),
}

impl<T> MaybeSendWrapperOption<T>
where
    T: Send + Sync,
{
    /// Create a new threadsafe value.
    pub fn new(value: Option<T>) -> Self {
        Self {
            inner: Inner::Threadsafe(value),
        }
    }
}

impl<T> MaybeSendWrapperOption<T> {
    /// Create a new non-threadsafe value.
    pub fn new_local(value: Option<T>) -> Self {
        Self {
            inner: if let Some(value) = value {
                Inner::Local(Some(SendWrapper::new(Some(value))))
            } else {
                Inner::Local(None)
            },
        }
    }

    /// Update a value in place with a callback.
    ///
    /// # Panics
    /// If the value is [`Inner::Local`] and it is called from a different thread than the one the instance has been created with, it will panic.
    pub fn update(&mut self, cb: impl FnOnce(&mut Option<T>)) {
        match &mut self.inner {
            Inner::Threadsafe(value) => cb(value),
            Inner::Local(value) => match value {
                Some(sw) => {
                    cb(sw.deref_mut());
                    if sw.is_none() {
                        *value = None;
                    }
                }
                None => {
                    let mut inner = None;
                    cb(&mut inner);
                    if let Some(inner) = inner {
                        *value = Some(SendWrapper::new(Some(inner)));
                    }
                }
            },
        }
    }

    /// Consume the value.
    ///
    /// # Panics
    /// Panics if the [`Inner::Local`] variant and it is called from a different thread than the one the instance has been created with.
    pub fn take(self) -> Option<T> {
        match self.inner {
            Inner::Threadsafe(value) => value,
            Inner::Local(value) => value.and_then(|value| value.take()),
        }
    }
}

impl<T> Deref for MaybeSendWrapperOption<T> {
    type Target = Option<T>;

    fn deref(&self) -> &Self::Target {
        match &self.inner {
            Inner::Threadsafe(value) => value,
            Inner::Local(value) => match value {
                Some(value) => value.deref(),
                None => &None,
            },
        }
    }
}

impl<T> DerefMut for MaybeSendWrapperOption<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match &mut self.inner {
            Inner::Threadsafe(value) => value,
            Inner::Local(value) => match value {
                Some(value) => value.deref_mut(),
                None => {
                    *value = Some(SendWrapper::new(None));
                    value.as_mut().unwrap().deref_mut()
                }
            },
        }
    }
}

impl<T: Debug> Debug for MaybeSendWrapperOption<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self.inner {
            Inner::Threadsafe(value) => {
                write!(f, "MaybeSendWrapperOption::Threadsafe({:?})", value)
            }
            Inner::Local(value) => {
                write!(f, "MaybeSendWrapperOption::Local({:?})", value)
            }
        }
    }
}

impl<T: Clone> Clone for MaybeSendWrapperOption<T> {
    fn clone(&self) -> Self {
        Self {
            inner: match &self.inner {
                Inner::Threadsafe(value) => Inner::Threadsafe(value.clone()),
                Inner::Local(value) => Inner::Local(value.clone()),
            },
        }
    }
}
