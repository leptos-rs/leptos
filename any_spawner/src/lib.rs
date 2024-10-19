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
//! ```rust
//! use any_spawner::Executor;
//!
//! // make sure an Executor has been initialized with one of the init_ functions
//!
//! # if false {
//! // spawn a thread-safe Future
//! Executor::spawn(async { /* ... */ });
//!
//! // spawn a Future that is !Send
//! Executor::spawn_local(async { /* ... */ });
//! # }
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

static SPAWN: OnceLock<fn(PinnedFuture<()>)> = OnceLock::new();
static SPAWN_LOCAL: OnceLock<fn(PinnedLocalFuture<()>)> = OnceLock::new();
static POLL_LOCAL: OnceLock<fn()> = OnceLock::new();

/// Errors that can occur when using the executor.
#[derive(Error, Debug)]
pub enum ExecutorError {
    /// The executor has already been set.
    #[error("Executor has already been set.")]
    AlreadySet,
}

/// A global async executor that can spawn tasks.
pub struct Executor;

impl Executor {
    /// Spawns a thread-safe [`Future`].
    /// ```rust
    /// use any_spawner::Executor;
    /// # if false {
    /// // spawn a thread-safe Future
    /// Executor::spawn(async { /* ... */ });
    /// # }
    /// ```
    #[track_caller]
    pub fn spawn(fut: impl Future<Output = ()> + Send + 'static) {
        if let Some(spawner) = SPAWN.get() {
            spawner(Box::pin(fut))
        } else {
            #[cfg(all(debug_assertions, feature = "tracing"))]
            tracing::error!(
                "At {}, tried to spawn a Future with Executor::spawn() before \
                 the Executor had been set.",
                std::panic::Location::caller()
            );
            #[cfg(all(debug_assertions, not(feature = "tracing")))]
            panic!(
                "At {}, tried to spawn a Future with Executor::spawn() before \
                 the Executor had been set.",
                std::panic::Location::caller()
            );
        }
    }

    /// Spawns a [`Future`] that cannot be sent across threads.
    /// ```rust
    /// use any_spawner::Executor;
    ///
    /// # if false {
    /// // spawn a thread-safe Future
    /// Executor::spawn_local(async { /* ... */ });
    /// # }
    /// ```
    #[track_caller]
    pub fn spawn_local(fut: impl Future<Output = ()> + 'static) {
        if let Some(spawner) = SPAWN_LOCAL.get() {
            spawner(Box::pin(fut))
        } else {
            #[cfg(all(debug_assertions, feature = "tracing"))]
            tracing::error!(
                "At {}, tried to spawn a Future with Executor::spawn_local() \
                 before the Executor had been set.",
                std::panic::Location::caller()
            );
            #[cfg(all(debug_assertions, not(feature = "tracing")))]
            panic!(
                "At {}, tried to spawn a Future with Executor::spawn_local() \
                 before the Executor had been set.",
                std::panic::Location::caller()
            );
        }
    }

    /// Waits until the next "tick" of the current async executor.
    pub async fn tick() {
        let (tx, rx) = futures::channel::oneshot::channel();
        Executor::spawn(async move {
            _ = tx.send(());
        });
        _ = rx.await;
    }

    /// Polls the current async executor.
    /// Not all async executors support polling, so this function may not do anything.
    pub fn poll_local() {
        if let Some(poller) = POLL_LOCAL.get() {
            poller()
        }
    }
}

impl Executor {
    /// Globally sets the [`tokio`] runtime as the executor used to spawn tasks.
    ///
    /// Returns `Err(_)` if an executor has already been set.
    ///
    /// Requires the `tokio` feature to be activated on this crate.
    #[cfg(feature = "tokio")]
    #[cfg_attr(docsrs, doc(cfg(feature = "tokio")))]
    pub fn init_tokio() -> Result<(), ExecutorError> {
        SPAWN
            .set(|fut| {
                tokio::spawn(fut);
            })
            .map_err(|_| ExecutorError::AlreadySet)?;
        SPAWN_LOCAL
            .set(|fut| {
                tokio::task::spawn_local(fut);
            })
            .map_err(|_| ExecutorError::AlreadySet)?;
        Ok(())
    }

    /// Globally sets the [`wasm-bindgen-futures`] runtime as the executor used to spawn tasks.
    ///
    /// Returns `Err(_)` if an executor has already been set.
    ///
    /// Requires the `wasm-bindgen` feature to be activated on this crate.
    #[cfg(feature = "wasm-bindgen")]
    #[cfg_attr(docsrs, doc(cfg(feature = "wasm-bindgen")))]
    pub fn init_wasm_bindgen() -> Result<(), ExecutorError> {
        SPAWN
            .set(|fut| {
                wasm_bindgen_futures::spawn_local(fut);
            })
            .map_err(|_| ExecutorError::AlreadySet)?;
        SPAWN_LOCAL
            .set(|fut| {
                wasm_bindgen_futures::spawn_local(fut);
            })
            .map_err(|_| ExecutorError::AlreadySet)?;
        Ok(())
    }

    /// Globally sets the [`glib`] runtime as the executor used to spawn tasks.
    ///
    /// Returns `Err(_)` if an executor has already been set.
    ///
    /// Requires the `glib` feature to be activated on this crate.
    #[cfg(feature = "glib")]
    #[cfg_attr(docsrs, doc(cfg(feature = "glib")))]
    pub fn init_glib() -> Result<(), ExecutorError> {
        SPAWN
            .set(|fut| {
                let main_context = glib::MainContext::default();
                main_context.spawn(fut);
            })
            .map_err(|_| ExecutorError::AlreadySet)?;
        SPAWN_LOCAL
            .set(|fut| {
                let main_context = glib::MainContext::default();
                main_context.spawn_local(fut);
            })
            .map_err(|_| ExecutorError::AlreadySet)?;
        Ok(())
    }

    /// Globally sets the [`futures`] executor as the executor used to spawn tasks,
    /// lazily creating a thread pool to spawn tasks into.
    ///
    /// Returns `Err(_)` if an executor has already been set.
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

        static THREAD_POOL: OnceLock<ThreadPool> = OnceLock::new();
        thread_local! {
            static LOCAL_POOL: RefCell<LocalPool> = RefCell::new(LocalPool::new());
            static SPAWNER: LocalSpawner = LOCAL_POOL.with(|pool| pool.borrow().spawner());
        }

        fn get_thread_pool() -> &'static ThreadPool {
            THREAD_POOL.get_or_init(|| {
                ThreadPool::new()
                    .expect("could not create futures executor ThreadPool")
            })
        }

        SPAWN
            .set(|fut| {
                get_thread_pool()
                    .spawn(fut)
                    .expect("failed to spawn future");
            })
            .map_err(|_| ExecutorError::AlreadySet)?;
        SPAWN_LOCAL
            .set(|fut| {
                SPAWNER.with(|spawner| {
                    spawner.spawn_local(fut).expect("failed to spawn future");
                });
            })
            .map_err(|_| ExecutorError::AlreadySet)?;
        POLL_LOCAL
            .set(|| {
                LOCAL_POOL.with(|pool| {
                    if let Ok(mut pool) = pool.try_borrow_mut() {
                        pool.run_until_stalled();
                    }
                    // If we couldn't borrow_mut, we're in a nested call to poll, so we don't need to do anything.
                });
            })
            .map_err(|_| ExecutorError::AlreadySet)?;
        Ok(())
    }

    /// Globally sets the [`async_executor`] executor as the executor used to spawn tasks,
    /// lazily creating a thread pool to spawn tasks into.
    ///
    /// Returns `Err(_)` if an executor has already been set.
    ///
    /// Requires the `async-executor` feature to be activated on this crate.
    #[cfg(feature = "async-executor")]
    #[cfg_attr(docsrs, doc(cfg(feature = "async-executor")))]
    pub fn init_async_executor() -> Result<(), ExecutorError> {
        use async_executor::{Executor, LocalExecutor};

        static THREAD_POOL: OnceLock<Executor> = OnceLock::new();
        thread_local! {
            static LOCAL_POOL: LocalExecutor<'static> = const { LocalExecutor::new() };
        }

        fn get_thread_pool() -> &'static Executor<'static> {
            THREAD_POOL.get_or_init(Executor::new)
        }

        SPAWN
            .set(|fut| {
                get_thread_pool().spawn(fut).detach();
            })
            .map_err(|_| ExecutorError::AlreadySet)?;
        SPAWN_LOCAL
            .set(|fut| {
                LOCAL_POOL.with(|pool| pool.spawn(fut).detach());
            })
            .map_err(|_| ExecutorError::AlreadySet)?;
        POLL_LOCAL
            .set(|| {
                LOCAL_POOL.with(|pool| pool.try_tick());
            })
            .map_err(|_| ExecutorError::AlreadySet)?;
        Ok(())
    }

    /// Globally sets a custom executor as the executor used to spawn tasks.
    ///
    /// Returns `Err(_)` if an executor has already been set.
    pub fn init_custom_executor(
        custom_executor: impl CustomExecutor + 'static,
    ) -> Result<(), ExecutorError> {
        static EXECUTOR: OnceLock<Box<dyn CustomExecutor>> = OnceLock::new();
        EXECUTOR
            .set(Box::new(custom_executor))
            .map_err(|_| ExecutorError::AlreadySet)?;

        SPAWN
            .set(|fut| {
                EXECUTOR.get().unwrap().spawn(fut);
            })
            .map_err(|_| ExecutorError::AlreadySet)?;
        SPAWN_LOCAL
            .set(|fut| EXECUTOR.get().unwrap().spawn_local(fut))
            .map_err(|_| ExecutorError::AlreadySet)?;
        POLL_LOCAL
            .set(|| EXECUTOR.get().unwrap().poll_local())
            .map_err(|_| ExecutorError::AlreadySet)?;
        Ok(())
    }
}

/// A trait for custom executors.
/// Custom executors can be used to integrate with any executor that supports spawning futures.
///  
/// All methods can be called recursively.
pub trait CustomExecutor: Send + Sync {
    /// Spawns a future, usually on a thread pool.
    fn spawn(&self, fut: PinnedFuture<()>);
    /// Spawns a local future. May require calling `poll_local` to make progress.
    fn spawn_local(&self, fut: PinnedLocalFuture<()>);
    /// Polls the executor, if it supports polling.
    fn poll_local(&self);
}
