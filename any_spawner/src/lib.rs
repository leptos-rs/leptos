//! This crate makes it easier to write asynchronous code that is executor-agnostic, by providing a
//! utility that can be used to spawn tasks in a variety of executors.
//!
//! It only supports single executor per program, but that executor can be set at runtime, anywhere
//! in your crate (or an application that depends on it).
//!
//! This can be extended to support any executor or runtime that supports spawning [`Future`]s.
//!
//! This is a least common denominator implementation in many ways. Limitations include:
//! - setting an executor is a one-time, global action
//! - no "join handle" or other result is returned from the spawn
//! - the `Future` must output `()`
//!
//! ```no_run
//! use any_spawner::Executor;
//!
//! // make sure an Executor has been initialized with one of the init_ functions
//!
//! // spawn a thread-safe Future
//! Executor::spawn(async { /* ... */ });
//!
//! // spawn a Future that is !Send
//! Executor::spawn_local(async { /* ... */ });
//! ```

#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![cfg_attr(docsrs, feature(doc_cfg))]

use std::{future::Future, pin::Pin, sync::OnceLock};
use thiserror::Error;

/// A future that has been pinned.
pub type PinnedFuture<T> = Pin<Box<dyn Future<Output = T> + Send>>;
/// A future that has been pinned.
pub type PinnedLocalFuture<T> = Pin<Box<dyn Future<Output = T>>>;

// Type alias for the spawn function pointer.
type SpawnFn = fn(PinnedFuture<()>);
// Type alias for the spawn_local function pointer.
type SpawnLocalFn = fn(PinnedLocalFuture<()>);
// Type alias for the poll_local function pointer.
type PollLocalFn = fn();

/// Holds the function pointers for the current global executor.
#[derive(Clone, Copy)]
struct ExecutorFns {
    spawn: SpawnFn,
    spawn_local: SpawnLocalFn,
    poll_local: PollLocalFn,
}

// Use a single OnceLock to ensure atomic initialization of all functions.
static EXECUTOR_FNS: OnceLock<ExecutorFns> = OnceLock::new();

// No-op functions to use when an executor doesn't support a specific operation.
#[cfg(any(feature = "tokio", feature = "wasm-bindgen", feature = "glib"))]
#[cold]
#[inline(never)]
fn no_op_poll() {}

#[cfg(all(not(feature = "wasm-bindgen"), not(debug_assertions)))]
#[cold]
#[inline(never)]
fn no_op_spawn(_: PinnedFuture<()>) {
    #[cfg(debug_assertions)]
    eprintln!(
        "Warning: Executor::spawn called, but no global 'spawn' function is \
         configured (perhaps only spawn_local is supported, e.g., on wasm \
         without threading?)."
    );
}

// Wasm panics if you spawn without an executor
#[cfg(feature = "wasm-bindgen")]
#[cold]
#[inline(never)]
fn no_op_spawn(_: PinnedFuture<()>) {
    panic!(
        "Executor::spawn called, but no global 'spawn' function is configured."
    );
}

#[cfg(not(debug_assertions))]
#[cold]
#[inline(never)]
fn no_op_spawn_local(_: PinnedLocalFuture<()>) {
    panic!(
        "Executor::spawn_local called, but no global 'spawn_local' function \
         is configured."
    );
}

/// Errors that can occur when using the executor.
#[derive(Error, Debug)]
pub enum ExecutorError {
    /// The executor has already been set.
    #[error("Global executor has already been set.")]
    AlreadySet,
}

/// A global async executor that can spawn tasks.
pub struct Executor;

impl Executor {
    /// Spawns a thread-safe [`Future`].
    ///
    /// Uses the globally configured executor.
    /// Panics if no global executor has been initialized.
    #[inline(always)]
    #[track_caller]
    pub fn spawn(fut: impl Future<Output = ()> + Send + 'static) {
        let pinned_fut = Box::pin(fut);

        if let Some(fns) = EXECUTOR_FNS.get() {
            (fns.spawn)(pinned_fut)
        } else {
            // No global executor set.
            handle_uninitialized_spawn(pinned_fut);
        }
    }

    /// Spawns a [`Future`] that cannot be sent across threads.
    ///
    /// Uses the globally configured executor.
    /// Panics if no global executor has been initialized.
    #[inline(always)]
    #[track_caller]
    pub fn spawn_local(fut: impl Future<Output = ()> + 'static) {
        let pinned_fut = Box::pin(fut);

        if let Some(fns) = EXECUTOR_FNS.get() {
            (fns.spawn_local)(pinned_fut)
        } else {
            // No global executor set.
            handle_uninitialized_spawn_local(pinned_fut);
        }
    }

    /// Waits until the next "tick" of the current async executor.
    /// Respects the global executor.
    #[inline(always)]
    pub async fn tick() {
        let (tx, rx) = futures::channel::oneshot::channel();
        #[cfg(not(all(feature = "wasm-bindgen", target_family = "wasm")))]
        Executor::spawn(async move {
            _ = tx.send(());
        });
        #[cfg(all(feature = "wasm-bindgen", target_family = "wasm"))]
        Executor::spawn_local(async move {
            _ = tx.send(());
        });

        _ = rx.await;
    }

    /// Polls the global async executor.
    ///
    /// Uses the globally configured executor.
    /// Does nothing if the global executor does not support polling.
    #[inline(always)]
    pub fn poll_local() {
        if let Some(fns) = EXECUTOR_FNS.get() {
            (fns.poll_local)()
        }
        // If not initialized or doesn't support polling, do nothing gracefully.
    }
}

impl Executor {
    /// Globally sets the [`tokio`] runtime as the executor used to spawn tasks.
    ///
    /// Returns `Err(_)` if a global executor has already been set.
    ///
    /// Requires the `tokio` feature to be activated on this crate.
    #[cfg(feature = "tokio")]
    #[cfg_attr(docsrs, doc(cfg(feature = "tokio")))]
    pub fn init_tokio() -> Result<(), ExecutorError> {
        let executor_impl = ExecutorFns {
            spawn: |fut| {
                tokio::spawn(fut);
            },
            spawn_local: |fut| {
                tokio::task::spawn_local(fut);
            },
            // Tokio doesn't have an explicit global poll function like LocalPool::run_until_stalled
            poll_local: no_op_poll,
        };
        EXECUTOR_FNS
            .set(executor_impl)
            .map_err(|_| ExecutorError::AlreadySet)
    }

    /// Globally sets the [`wasm-bindgen-futures`] runtime as the executor used to spawn tasks.
    ///
    /// Returns `Err(_)` if a global executor has already been set.
    ///
    /// Requires the `wasm-bindgen` feature to be activated on this crate.
    #[cfg(feature = "wasm-bindgen")]
    #[cfg_attr(docsrs, doc(cfg(feature = "wasm-bindgen")))]
    pub fn init_wasm_bindgen() -> Result<(), ExecutorError> {
        let executor_impl = ExecutorFns {
            // wasm-bindgen-futures only supports spawn_local
            spawn: no_op_spawn,
            spawn_local: |fut| {
                wasm_bindgen_futures::spawn_local(fut);
            },
            poll_local: no_op_poll,
        };
        EXECUTOR_FNS
            .set(executor_impl)
            .map_err(|_| ExecutorError::AlreadySet)
    }

    /// Globally sets the [`glib`] runtime as the executor used to spawn tasks.
    ///
    /// Returns `Err(_)` if a global executor has already been set.
    ///
    /// Requires the `glib` feature to be activated on this crate.
    #[cfg(feature = "glib")]
    #[cfg_attr(docsrs, doc(cfg(feature = "glib")))]
    pub fn init_glib() -> Result<(), ExecutorError> {
        let executor_impl = ExecutorFns {
            spawn: |fut| {
                let main_context = glib::MainContext::default();
                main_context.spawn(fut);
            },
            spawn_local: |fut| {
                let main_context = glib::MainContext::default();
                main_context.spawn_local(fut);
            },
            // Glib needs event loop integration, explicit polling isn't the standard model here.
            poll_local: no_op_poll,
        };
        EXECUTOR_FNS
            .set(executor_impl)
            .map_err(|_| ExecutorError::AlreadySet)
    }

    /// Globally sets the [`futures`] executor as the executor used to spawn tasks,
    /// lazily creating a thread pool to spawn tasks into.
    ///
    /// Returns `Err(_)` if a global executor has already been set.
    ///
    /// Requires the `futures-executor` feature to be activated on this crate.
    #[cfg(feature = "futures-executor")]
    #[cfg_attr(docsrs, doc(cfg(feature = "futures-executor")))]
    pub fn init_futures_executor() -> Result<(), ExecutorError> {
        use futures::{
            executor::{LocalPool, LocalSpawner, ThreadPool},
            task::{LocalSpawnExt, SpawnExt},
        };
        use std::cell::RefCell;

        // Keep the lazy-init ThreadPool and thread-local LocalPool for spawn_local impl
        static THREAD_POOL: OnceLock<ThreadPool> = OnceLock::new();
        thread_local! {
            static LOCAL_POOL: RefCell<LocalPool> = RefCell::new(LocalPool::new());
            // SPAWNER is derived from LOCAL_POOL, keep it for efficiency inside the closure
            static SPAWNER: LocalSpawner = LOCAL_POOL.with(|pool| pool.borrow().spawner());
        }

        fn get_thread_pool() -> &'static ThreadPool {
            THREAD_POOL.get_or_init(|| {
                ThreadPool::new()
                    .expect("could not create futures executor ThreadPool")
            })
        }

        let executor_impl = ExecutorFns {
            spawn: |fut| {
                get_thread_pool()
                    .spawn(fut)
                    .expect("failed to spawn future on ThreadPool");
            },
            spawn_local: |fut| {
                // Use the thread_local SPAWNER derived from LOCAL_POOL
                SPAWNER.with(|spawner| {
                    spawner
                        .spawn_local(fut)
                        .expect("failed to spawn local future");
                });
            },
            poll_local: || {
                // Use the thread_local LOCAL_POOL
                LOCAL_POOL.with(|pool| {
                    // Use try_borrow_mut to prevent panic during re-entrant calls
                    if let Ok(mut pool) = pool.try_borrow_mut() {
                        pool.run_until_stalled();
                    }
                    // If already borrowed, we're likely in a nested poll, so do nothing.
                });
            },
        };

        EXECUTOR_FNS
            .set(executor_impl)
            .map_err(|_| ExecutorError::AlreadySet)
    }

    /// Globally sets the [`async_executor`] executor as the executor used to spawn tasks,
    /// lazily creating a thread pool to spawn tasks into.
    ///
    /// Returns `Err(_)` if a global executor has already been set.
    ///
    /// Requires the `async-executor` feature to be activated on this crate.
    #[cfg(feature = "async-executor")]
    #[cfg_attr(docsrs, doc(cfg(feature = "async-executor")))]
    pub fn init_async_executor() -> Result<(), ExecutorError> {
        use async_executor::{Executor as AsyncExecutor, LocalExecutor};

        // Keep the lazy-init global Executor and thread-local LocalExecutor for spawn_local impl
        static ASYNC_EXECUTOR: OnceLock<AsyncExecutor<'static>> =
            OnceLock::new();
        thread_local! {
            static LOCAL_EXECUTOR_POOL: LocalExecutor<'static> = const { LocalExecutor::new() };
        }

        fn get_async_executor() -> &'static AsyncExecutor<'static> {
            ASYNC_EXECUTOR.get_or_init(AsyncExecutor::new)
        }

        let executor_impl = ExecutorFns {
            spawn: |fut| {
                get_async_executor().spawn(fut).detach();
            },
            spawn_local: |fut| {
                LOCAL_EXECUTOR_POOL.with(|pool| pool.spawn(fut).detach());
            },
            poll_local: || {
                LOCAL_EXECUTOR_POOL.with(|pool| {
                    // try_tick polls the local executor without blocking
                    // This prevents issues if called recursively or from within a task.
                    pool.try_tick();
                });
            },
        };
        EXECUTOR_FNS
            .set(executor_impl)
            .map_err(|_| ExecutorError::AlreadySet)
    }

    /// Globally sets a custom executor as the executor used to spawn tasks.
    ///
    /// Requires the custom executor to be `Send + Sync` as it will be stored statically.
    ///
    /// Returns `Err(_)` if a global executor has already been set.
    pub fn init_custom_executor(
        custom_executor: impl CustomExecutor + Send + Sync + 'static,
    ) -> Result<(), ExecutorError> {
        // Store the custom executor instance itself to call its methods.
        // Use Box for dynamic dispatch.
        static CUSTOM_EXECUTOR_INSTANCE: OnceLock<
            Box<dyn CustomExecutor + Send + Sync>,
        > = OnceLock::new();

        CUSTOM_EXECUTOR_INSTANCE
            .set(Box::new(custom_executor))
            .map_err(|_| ExecutorError::AlreadySet)?;

        // Now set the ExecutorFns using the stored instance
        let executor_impl = ExecutorFns {
            spawn: |fut| {
                // Unwrap is safe because we just set it successfully or returned Err.
                CUSTOM_EXECUTOR_INSTANCE.get().unwrap().spawn(fut);
            },
            spawn_local: |fut| {
                CUSTOM_EXECUTOR_INSTANCE.get().unwrap().spawn_local(fut);
            },
            poll_local: || {
                CUSTOM_EXECUTOR_INSTANCE.get().unwrap().poll_local();
            },
        };

        EXECUTOR_FNS
            .set(executor_impl)
            .map_err(|_| ExecutorError::AlreadySet)
        // If setting EXECUTOR_FNS fails (extremely unlikely race if called *concurrently*
        // with another init_* after CUSTOM_EXECUTOR_INSTANCE was set), we technically
        // leave CUSTOM_EXECUTOR_INSTANCE set but EXECUTOR_FNS not. This is an edge case,
        // but the primary race condition is solved.
    }

    /// Sets a custom executor *for the current thread only*.
    ///
    /// This overrides the global executor for calls to `spawn`, `spawn_local`, and `poll_local`
    /// made *from the current thread*. It does not affect other threads or the global state.
    ///
    /// The provided `custom_executor` must implement [`CustomExecutor`] and `'static`, but does
    /// **not** need to be `Send` or `Sync`.
    ///
    /// Returns `Err(ExecutorError::AlreadySet)` if a *local* executor has already been set
    /// *for this thread*.
    pub fn init_local_custom_executor(
        custom_executor: impl CustomExecutor + 'static,
    ) -> Result<(), ExecutorError> {
        // Store the custom executor instance itself to call its methods.
        // Use Box for dynamic dispatch.
        thread_local! {
            static CUSTOM_EXECUTOR_INSTANCE: OnceLock<
                Box<dyn CustomExecutor>,
            > = OnceLock::new();
        };

        CUSTOM_EXECUTOR_INSTANCE.with(|this| {
            this.set(Box::new(custom_executor))
                .map_err(|_| ExecutorError::AlreadySet)
        })?;

        // Now set the ExecutorFns using the stored instance
        let executor_impl = ExecutorFns {
            spawn: |fut| {
                // Unwrap is safe because we just set it successfully or returned Err.
                CUSTOM_EXECUTOR_INSTANCE
                    .with(|this| this.get().unwrap().spawn(fut));
            },
            spawn_local: |fut| {
                CUSTOM_EXECUTOR_INSTANCE
                    .with(|this| this.get().unwrap().spawn_local(fut));
            },
            poll_local: || {
                CUSTOM_EXECUTOR_INSTANCE
                    .with(|this| this.get().unwrap().poll_local());
            },
        };

        EXECUTOR_FNS
            .set(executor_impl)
            .map_err(|_| ExecutorError::AlreadySet)
    }
}

/// A trait for custom executors.
/// Custom executors can be used to integrate with any executor that supports spawning futures.
///
/// If used with `init_custom_executor`, the implementation must be `Send + Sync + 'static`.
///
/// All methods can be called recursively. Implementors should be mindful of potential
/// deadlocks or excessive resource consumption if recursive calls are not handled carefully
/// (e.g., using `try_borrow_mut` or non-blocking polls within implementations).
pub trait CustomExecutor {
    /// Spawns a future, usually on a thread pool.
    fn spawn(&self, fut: PinnedFuture<()>);
    /// Spawns a local future. May require calling `poll_local` to make progress.
    fn spawn_local(&self, fut: PinnedLocalFuture<()>);
    /// Polls the executor, if it supports polling. Implementations should ideally be
    /// non-blocking or use mechanisms like `try_tick` or `try_borrow_mut` to handle
    /// re-entrant calls safely.
    fn poll_local(&self);
}

// Ensure CustomExecutor is object-safe
#[allow(dead_code)]
fn test_object_safety(_: Box<dyn CustomExecutor + Send + Sync>) {} // Added Send + Sync constraint here for global usage

/// Handles the case where `Executor::spawn` is called without an initialized executor.
#[cold] // Less likely path
#[inline(never)]
#[track_caller]
fn handle_uninitialized_spawn(_fut: PinnedFuture<()>) {
    let caller = std::panic::Location::caller();
    #[cfg(all(debug_assertions, feature = "tracing"))]
    {
        tracing::error!(
            target: "any_spawner",
            spawn_caller=%caller,
            "Executor::spawn called before a global executor was initialized. Task dropped."
        );
        // Drop the future implicitly after logging
        drop(_fut);
    }
    #[cfg(all(debug_assertions, not(feature = "tracing")))]
    {
        panic!(
            "At {caller}, tried to spawn a Future with Executor::spawn() \
             before a global executor was initialized."
        );
    }
    // In release builds (without tracing), call the specific no-op function.
    #[cfg(not(debug_assertions))]
    {
        no_op_spawn(_fut);
    }
}

/// Handles the case where `Executor::spawn_local` is called without an initialized executor.
#[cold] // Less likely path
#[inline(never)]
#[track_caller]
fn handle_uninitialized_spawn_local(_fut: PinnedLocalFuture<()>) {
    let caller = std::panic::Location::caller();
    #[cfg(all(debug_assertions, feature = "tracing"))]
    {
        tracing::error!(
            target: "any_spawner",
            spawn_caller=%caller,
            "Executor::spawn_local called before a global executor was initialized. \
            Task likely dropped or panicked."
        );
        // Fall through to panic or no-op depending on build/target
    }
    #[cfg(all(debug_assertions, not(feature = "tracing")))]
    {
        panic!(
            "At {caller}, tried to spawn a Future with \
             Executor::spawn_local() before a global executor was initialized."
        );
    }
    // In release builds (without tracing), call the specific no-op function (which usually panics).
    #[cfg(not(debug_assertions))]
    {
        no_op_spawn_local(_fut);
    }
}
