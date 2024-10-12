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

use core::{future::Future, panic::Location, pin::Pin};
use futures::channel::oneshot;
use std::sync::OnceLock;
use thiserror::Error;

/// Helper alias for pinned Box holding a Future so `poll` can be called.
pub(crate) type PinnedFuture<T> = Pin<Box<dyn Future<Output = T> + Send>>;

/// Helper alias for pinned Box holding a Future so `poll` can be called.
///
/// Unlike [`PinnedFuture`], the boxed future is not [`Send`], that is,
/// the future can only be run on the local thread.
pub(crate) type PinnedLocalFuture<T> = Pin<Box<dyn Future<Output = T>>>;

/// Handle to spawn a new [`PinnedFuture`] on the initiated [`Executor`].
static SPAWN: OnceLock<fn(PinnedFuture<()>)> = OnceLock::new();

/// Handle to spawn a new [`PinnedLocalFuture`] on the initiated [`Executor`].
///
/// It is useful when you have a Future that is not [`Send`].
static SPAWN_LOCAL: OnceLock<fn(PinnedLocalFuture<()>)> = OnceLock::new();

/// Handle to actually run the initiated [`Executor`].
///
/// At the moment, it is only useful when [`futures`] is the chosen runtime
/// since, unlike [`tokio`] and [`glib`], it is not expected to be
/// initiated with an ambiant / global executor. This means the library
/// users are expected to run the initiated [`Executor`] at some point
/// for the spawned [`Future`]s to actually run.
static RUN: OnceLock<fn()> = OnceLock::new();

/// Errors that can occur when using the executor.
#[derive(Error, Debug)]
#[non_exhaustive]
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
    #[inline]
    pub fn spawn<T>(fut: T)
    where
        T: Future<Output = ()> + Send + 'static,
    {
        if let Some(spawner) = SPAWN.get() {
            spawner(Box::pin(fut))
        } else {
            #[cfg(all(debug_assertions, feature = "tracing"))]
            tracing::error!(
                "At {}, tried to spawn a Future with Executor::spawn() before \
                 the Executor had been set.",
                Location::caller()
            );
            #[cfg(all(debug_assertions, not(feature = "tracing")))]
            panic!(
                "At {}, tried to spawn a Future with Executor::spawn() before \
                 the Executor had been set.",
                Location::caller()
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
    #[inline]
    pub fn spawn_local<T>(fut: T)
    where
        T: Future<Output = ()> + 'static,
    {
        if let Some(spawner) = SPAWN_LOCAL.get() {
            spawner(Box::pin(fut))
        } else {
            #[cfg(all(debug_assertions, feature = "tracing"))]
            tracing::error!(
                "At {}, tried to spawn a Future with Executor::spawn_local() \
                 before the Executor had been set.",
                Location::caller()
            );
            #[cfg(all(debug_assertions, not(feature = "tracing")))]
            panic!(
                "At {}, tried to spawn a Future with Executor::spawn_local() \
                 before the Executor had been set.",
                Location::caller()
            );
        }
    }

    /// Run the [`Executor`].
    #[track_caller]
    #[inline]
    pub fn run() {
        if let Some(run) = RUN.get() {
            run();
        } else {
            #[cfg(all(debug_assertions, feature = "tracing"))]
            tracing::error!(
                "At {}, tried to run an executor with Executor::run() \
                 before the Executor had been set.",
                Location::caller()
            );
            #[cfg(all(debug_assertions, not(feature = "tracing")))]
            panic!(
                "At {}, tried to run an executor with Executor::run() \
                 before the Executor had been set.",
                Location::caller()
            );
        }
    }

    /// Waits until the next "tick" of the current async executor.
    #[inline]
    pub async fn tick() {
        let (tx, rx) = oneshot::channel();
        Self::spawn(async move {
            _ = tx.send(());
        });
        _ = rx.await;
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
            executor::{LocalPool, ThreadPool},
            task::{LocalSpawnExt, SpawnExt},
        };

        static THREAD_POOL: OnceLock<ThreadPool> = OnceLock::new();
        thread_local! {
            static LOCAL_POOL: LocalPool = LocalPool::new();
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
                LOCAL_POOL.with(|pool| {
                    let spawner = pool.spawner();
                    spawner.spawn_local(fut).expect("failed to spawn future");
                });
            })
            .map_err(|_| ExecutorError::AlreadySet)?;
        Ok(())
    }

    /// Globally sets the [`futures`] executor as the executor used to spawn tasks,
    /// using a single-threaded local thread pool, useful for platforms without
    /// multi-threading capabilities such as `wasm32-wasip1`.
    ///
    /// Returns `Err(_)` if an executor has already been set.
    ///
    /// Requires the `futures-executor` feature to be activated on this crate.
    #[cfg(feature = "futures-executor")]
    #[cfg_attr(docsrs, doc(cfg(feature = "futures-executor")))]
    pub fn init_futures_local_executor() -> Result<(), ExecutorError> {
        use std::cell::RefCell;

        use futures::{
            executor::{LocalPool, LocalSpawner},
            task::LocalSpawnExt,
        };

        thread_local! {
            static LOCAL_POOL: RefCell<LocalPool> = RefCell::new(LocalPool::new());
            static SPAWNER_HANDLE: OnceLock<LocalSpawner> = const { OnceLock::new() };
        }

        SPAWNER_HANDLE.with(|spawner_handle| {
            LOCAL_POOL.with(|local_pool| {
                spawner_handle
                    .set(local_pool.borrow().spawner())
                    .expect("unexpected error when getting executor spawner");
            });
        });

        SPAWN
            .set(|fut| {
                SPAWNER_HANDLE.with(|spawner| {
                    spawner
                        .get()
                        .expect("executor spawner was not set")
                        .spawn_local(fut)
                        .expect("failed to spawn future");
                });
            })
            .map_err(|_| ExecutorError::AlreadySet)?;
        SPAWN_LOCAL
            .set(|fut| {
                SPAWNER_HANDLE.with(|spawner| {
                    spawner
                        .get()
                        .expect("executor spawner was not set")
                        .spawn_local(fut)
                        .expect("failed to spawn future");
                });
            })
            .map_err(|_| ExecutorError::AlreadySet)?;

        RUN.set(|| {
            LOCAL_POOL.with(|pool| {
                pool.borrow_mut().run();
            });
        })
        .map_err(|_| ExecutorError::AlreadySet)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "futures-executor")]
    #[test]
    fn can_spawn_local_future() {
        use crate::Executor;
        use std::rc::Rc;
        Executor::init_futures_executor().expect("couldn't set executor");
        let rc = Rc::new(());
        Executor::spawn_local(async {
            _ = rc;
        });
        Executor::spawn(async {});
    }

    #[cfg(feature = "futures-executor")]
    #[test]
    fn can_spawn_future_single_thread() {
        use crate::Executor;
        Executor::init_futures_local_executor().expect("couldn't set executor");
        Executor::spawn_local(async {});
        Executor::spawn(async {});
    }
}
