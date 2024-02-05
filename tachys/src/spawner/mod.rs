use std::future::Future;

/// Allows spawning a [`Future`] to be run in a separate task.
pub trait Spawner {
    fn spawn<Fut>(fut: Fut)
    where
        Fut: Future + Send + Sync + 'static;

    fn spawn_local<Fut>(fut: Fut)
    where
        Fut: Future + 'static;
}

/// A spawner that will block in place when spawning an async task.
///
/// This is mostly useful for testing, and especially for synchronous-only code.
pub struct BlockSpawn;

impl Spawner for BlockSpawn {
    fn spawn<Fut>(fut: Fut)
    where
        Fut: Future + Send + Sync + 'static,
    {
        futures::executor::block_on(async move {
            fut.await;
        });
    }

    fn spawn_local<Fut>(fut: Fut)
    where
        Fut: Future + 'static,
    {
        futures::executor::block_on(async move {
            fut.await;
        });
    }
}

#[cfg(feature = "web")]
pub mod wasm {
    use super::Spawner;
    use wasm_bindgen_futures::spawn_local;

    #[derive(Debug, Copy, Clone)]
    pub struct Wasm;

    impl Spawner for Wasm {
        fn spawn<Fut>(fut: Fut)
        where
            Fut: futures::Future + Send + Sync + 'static,
        {
            Self::spawn_local(fut);
        }

        fn spawn_local<Fut>(fut: Fut)
        where
            Fut: futures::Future + 'static,
        {
            spawn_local(async move {
                fut.await;
            });
        }
    }
}

#[cfg(feature = "tokio")]
pub mod tokio {
    use super::Spawner;
    use tokio::task::{spawn, spawn_local};

    #[derive(Debug, Copy, Clone)]
    pub struct Tokio;

    impl Spawner for Tokio {
        fn spawn<Fut>(fut: Fut)
        where
            Fut: futures::Future + Send + Sync + 'static,
        {
            spawn(async move {
                fut.await;
            });
        }

        fn spawn_local<Fut>(fut: Fut)
        where
            Fut: futures::Future + 'static,
        {
            spawn_local(async move {
                fut.await;
            });
        }
    }
}
