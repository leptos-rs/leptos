//! Sets and uses a global `async` executor, which is used for scheduling effects.
//!
//! The executor must be set exactly once in a program.

use crate::{PinnedFuture, PinnedLocalFuture};
use std::{future::Future, sync::OnceLock};
use thiserror::Error;

static SPAWN: OnceLock<fn(PinnedFuture<()>)> = OnceLock::new();
static SPAWN_LOCAL: OnceLock<fn(PinnedLocalFuture<()>)> = OnceLock::new();

#[derive(Error, Debug)]
pub enum ExecutorError {
    #[error("Executor has already been set.")]
    AlreadySet,
}

pub struct Executor;

impl Executor {
    #[track_caller]
    pub fn spawn(fut: impl Future<Output = ()> + Send + Sync + 'static) {
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
}

impl Executor {
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
}
