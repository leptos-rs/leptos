#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! A utility library for wrapping arbitrary errors, and for “throwing” errors in a way
//! that can be caught by user-defined error hooks.

use std::{
    cell::RefCell,
    error,
    fmt::{self, Display},
    future::Future,
    ops,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

/* Wrapper Types */

/// This is a result type into which any error can be converted.
///
/// Results are stored as [`Error`].
pub type Result<T, E = Error> = core::result::Result<T, E>;

/// A generic wrapper for any error.
#[derive(Debug, Clone)]
#[repr(transparent)]
pub struct Error(Arc<dyn error::Error + Send + Sync>);

impl Error {
    /// Converts the wrapper into the inner reference-counted error.
    pub fn into_inner(self) -> Arc<dyn error::Error + Send + Sync> {
        Arc::clone(&self.0)
    }
}

impl ops::Deref for Error {
    type Target = Arc<dyn error::Error + Send + Sync>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl<T> From<T> for Error
where
    T: error::Error + Send + Sync + 'static,
{
    fn from(value: T) -> Self {
        Error(Arc::new(value))
    }
}

/// Implements behavior that allows for global or scoped error handling.
///
/// This allows for both "throwing" errors to register them, and "clearing" errors when they are no
/// longer valid. This is useful for something like a user interface, in which an error can be
/// "thrown" on some invalid user input, and later "cleared" if the user corrects the input.
/// Keeping a unique identifier for each error allows the UI to be updated accordingly.
pub trait ErrorHook: Send + Sync {
    /// Handles the given error, returning a unique identifier.
    fn throw(&self, error: Error) -> ErrorId;

    /// Clears the error associated with the given identifier.
    fn clear(&self, id: &ErrorId);
}

/// A unique identifier for an error. This is returned when you call [`throw`], which calls a
/// global error handler.
#[derive(Debug, PartialEq, Eq, Hash, Clone, Default)]
pub struct ErrorId(usize);

impl Display for ErrorId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.0, f)
    }
}

impl From<usize> for ErrorId {
    fn from(value: usize) -> Self {
        Self(value)
    }
}

thread_local! {
    static ERROR_HOOK: RefCell<Option<Arc<dyn ErrorHook>>> = RefCell::new(None);
}

/// Resets the error hook to its previous state when dropped.
pub struct ResetErrorHookOnDrop(Option<Arc<dyn ErrorHook>>);

impl Drop for ResetErrorHookOnDrop {
    fn drop(&mut self) {
        ERROR_HOOK.with_borrow_mut(|this| *this = self.0.take())
    }
}

/// Returns the current error hook.
pub fn get_error_hook() -> Option<Arc<dyn ErrorHook>> {
    ERROR_HOOK.with_borrow(Clone::clone)
}

/// Sets the current thread-local error hook, which will be invoked when [`throw`] is called.
pub fn set_error_hook(hook: Arc<dyn ErrorHook>) -> ResetErrorHookOnDrop {
    ResetErrorHookOnDrop(
        ERROR_HOOK.with_borrow_mut(|this| Option::replace(this, hook)),
    )
}

/// Invokes the error hook set by [`set_error_hook`] with the given error.
pub fn throw(error: impl Into<Error>) -> ErrorId {
    ERROR_HOOK
        .with_borrow(|hook| hook.as_ref().map(|hook| hook.throw(error.into())))
        .unwrap_or_default()
}

/// Clears the given error from the current error hook.
pub fn clear(id: &ErrorId) {
    ERROR_HOOK
        .with_borrow(|hook| hook.as_ref().map(|hook| hook.clear(id)))
        .unwrap_or_default()
}

pin_project_lite::pin_project! {
    /// A [`Future`] that reads the error hook that is set when it is created, and sets this as the
    /// current error hook whenever it is polled.
    pub struct ErrorHookFuture<Fut> {
        hook: Option<Arc<dyn ErrorHook>>,
        #[pin]
        inner: Fut
    }
}

impl<Fut> ErrorHookFuture<Fut> {
    /// Reads the current hook and wraps the given [`Future`], returning a new `Future` that will
    /// set the error hook whenever it is polled.
    pub fn new(inner: Fut) -> Self {
        Self {
            hook: ERROR_HOOK.with_borrow(Clone::clone),
            inner,
        }
    }
}

impl<Fut> Future for ErrorHookFuture<Fut>
where
    Fut: Future,
{
    type Output = Fut::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        let _hook = this
            .hook
            .as_ref()
            .map(|hook| set_error_hook(Arc::clone(hook)));
        this.inner.poll(cx)
    }
}
