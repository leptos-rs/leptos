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
/// - None case will not construct a SendWrapper, so no panics if None when dropping on a different thread.
pub struct MaybeSendWrapperOption<T> {
    inner: Inner<T>,
    none_fallback: Option<T>,
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
            none_fallback: None,
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
            none_fallback: None,
        }
    }
}

impl<T> Deref for MaybeSendWrapperOption<T> {
    type Target = Option<T>;

    fn deref(&self) -> &Self::Target {
        match &self.inner {
            Inner::Threadsafe(value) => value,
            Inner::Local(value) => {
                if value.is_some() {
                    value
                        .as_ref()
                        .expect("Internal Option always Some()")
                        .deref()
                } else {
                    &self.none_fallback
                }
            }
        }
    }
}

impl<T> DerefMut for MaybeSendWrapperOption<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match &mut self.inner {
            Inner::Threadsafe(value) => value,
            Inner::Local(value) => {
                if value.is_some() {
                    value
                        .as_mut()
                        .expect("Internal Option always Some()")
                        .deref_mut()
                } else {
                    &mut self.none_fallback
                }
            }
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
            none_fallback: None,
        }
    }
}
