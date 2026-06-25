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

/// A generic wrapper for any error.
#[derive(Debug, Clone)]
#[repr(transparent)]
pub struct Error(Arc<dyn error::Error + Send + Sync>);

impl Error {
    /// Wraps a concrete, sized error.
    pub fn new<E>(err: E) -> Self
    where
        E: error::Error + Send + Sync + 'static,
    {
        // `Arc<E>` coerces to `Arc<dyn Error + Send + Sync>` in place; no
        // intermediate `Box`, no reallocation.
        Error(Arc::new(err))
    }

    /// Converts the wrapper into the inner reference-counted error.
    pub fn into_inner(self) -> Arc<dyn error::Error + Send + Sync> {
        // Move the `Arc` out of the wrapper rather than cloning it: this
        // consumes `self`, so there is no reason to bump and then immediately
        // drop the reference count.
        self.0
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
    T: Into<Box<dyn error::Error + Send + Sync + 'static>>,
{
    fn from(value: T) -> Self {
        Error(Arc::from(value.into()))
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
        // Best-effort restore. If `ERROR_HOOK` is currently borrowed (e.g. this
        // guard is dropped from inside a hook callback that still holds the
        // borrow) or already destroyed (thread teardown), skip the restore
        // instead of panicking: a panic escaping `drop` during unwinding aborts
        // the whole process.
        let _ = ERROR_HOOK.try_with(|cell| {
            if let Ok(mut slot) = cell.try_borrow_mut() {
                *slot = self.0.take();
            }
        });
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
    // Clone the hook out of the thread-local *before* invoking it, so the
    // `RefCell` borrow is released for the duration of the callback. Holding the
    // borrow across the call would make any re-entrant `set_error_hook` /
    // `throw` / `clear` performed by the hook panic with `BorrowMutError`.
    get_error_hook()
        .map(|hook| hook.throw(error.into()))
        .unwrap_or_default()
}

/// Clears the given error from the current error hook.
pub fn clear(id: &ErrorId) {
    // See `throw`: release the borrow before calling into the hook so the hook
    // may itself touch the error-hook slot without panicking.
    if let Some(hook) = get_error_hook() {
        hook.clear(id);
    }
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
        // Install the hook captured at construction time for the duration of
        // this poll, *including* the `None` case. A future created before any
        // hook was set must clear the slot while polling rather than silently
        // inherit whatever hook happens to be installed on the polling thread.
        // The guard restores the previous hook when the poll returns.
        let _hook =
            ResetErrorHookOnDrop(ERROR_HOOK.with_borrow_mut(|cur| {
                std::mem::replace(cur, this.hook.clone())
            }));
        this.inner.poll(cx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error as StdError;

    #[derive(Debug)]
    struct MyError;

    impl Display for MyError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "MyError")
        }
    }

    impl StdError for MyError {}

    #[test]
    fn test_from() {
        let e = MyError;
        let _le = Error::from(e);

        let e = "some error".to_string();
        let _le = Error::from(e);

        let e = anyhow::anyhow!("anyhow error");
        let _le = Error::from(e);
    }

    #[test]
    fn new_wraps_concrete_error_in_single_arc() {
        // `Error::new` takes the concrete error type directly and dereferences
        // to the original error.
        let e = Error::new(MyError);
        assert_eq!(e.to_string(), "MyError");
        // Parity with the `From` path for a concrete error type.
        assert_eq!(e.to_string(), Error::from(MyError).to_string());
        // The freshly-constructed error is the sole owner of its `Arc`.
        let inner = e.into_inner();
        assert_eq!(Arc::strong_count(&inner), 1);
    }

    #[test]
    fn into_inner_yields_uniquely_owned_arc() {
        // `into_inner` consumes the only `Error`, so the returned `Arc` is the
        // sole owner of the underlying error.
        let e: Error = "boom".to_string().into();
        let inner = e.into_inner();
        assert_eq!(Arc::strong_count(&inner), 1);

        // With another owner alive, the count is exactly that owner plus the
        // returned `Arc`: `into_inner` introduces no extra references.
        let e: Error = "boom".to_string().into();
        let clone = e.clone();
        let inner = e.into_inner();
        assert_eq!(Arc::strong_count(&inner), 2);
        drop(clone);
        assert_eq!(Arc::strong_count(&inner), 1);
    }

    struct NoOpHook;
    impl ErrorHook for NoOpHook {
        fn throw(&self, _: Error) -> ErrorId {
            ErrorId::default()
        }
        fn clear(&self, _: &ErrorId) {}
    }

    // A hook that re-enters the error-hook machinery from inside its own
    // callbacks. Before the borrow was released ahead of the call, this
    // panicked with `BorrowMutError`.
    #[test]
    fn hook_may_reenter_error_hook_machinery_from_callback() {
        struct Reenter;
        impl ErrorHook for Reenter {
            fn throw(&self, _: Error) -> ErrorId {
                let _guard = set_error_hook(Arc::new(NoOpHook));
                ErrorId::default()
            }
            fn clear(&self, _id: &ErrorId) {
                // Re-enter the mutable path from `clear` as well.
                let _guard = set_error_hook(Arc::new(NoOpHook));
            }
        }

        let _guard = set_error_hook(Arc::new(Reenter));
        // Neither of these may panic.
        let id = throw("boom");
        clear(&id);
    }

    // A future built before any hook is installed captures `None`; polling it
    // must clear the slot for the duration of the poll instead of routing
    // `throw` to whatever hook happens to be installed on the polling thread.
    #[test]
    fn error_hook_future_with_no_captured_hook_clears_ambient_hook_during_poll()
    {
        use std::sync::atomic::{AtomicUsize, Ordering};

        struct Counter(AtomicUsize);
        impl ErrorHook for Counter {
            fn throw(&self, _: Error) -> ErrorId {
                self.0.fetch_add(1, Ordering::SeqCst);
                ErrorId::default()
            }
            fn clear(&self, _: &ErrorId) {}
        }

        // Built before any hook is installed -> captured hook is `None`.
        let fut = ErrorHookFuture::new(async {
            throw("inside future");
        });

        // Now install a counting hook on this thread.
        let counter = Arc::new(Counter(AtomicUsize::new(0)));
        let _g = set_error_hook(counter.clone());

        // Drive the future to completion. The `throw` inside it must NOT reach
        // the ambient `counter` hook, because the future captured `None`.
        let mut fut = std::pin::pin!(fut);
        let mut cx = Context::from_waker(std::task::Waker::noop());
        assert!(fut.as_mut().poll(&mut cx).is_ready());

        assert_eq!(counter.0.load(Ordering::SeqCst), 0);
        // The ambient hook is restored once the poll returns.
        assert!(get_error_hook().is_some());
    }

    // Dropping a `ResetErrorHookOnDrop` while the hook slot is already borrowed
    // must not panic. Before the guard used `try_borrow_mut`, this panicked
    // with `BorrowMutError` from inside `drop`.
    #[test]
    fn reset_guard_drop_is_silent_while_hook_slot_is_borrowed() {
        let guard = set_error_hook(Arc::new(NoOpHook));
        // Hold an immutable borrow of the slot, then drop the guard inside it.
        ERROR_HOOK.with_borrow(|_held| {
            drop(guard);
        });
    }
}
