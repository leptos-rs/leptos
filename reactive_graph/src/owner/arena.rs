use super::OWNER;
use or_poisoned::OrPoisoned;
use slotmap::{new_key_type, SlotMap};
#[cfg(not(feature = "sandboxed-arenas"))]
use std::sync::OnceLock;
use std::{any::Any, hash::Hash, marker::PhantomData, sync::RwLock};
#[cfg(feature = "sandboxed-arenas")]
use std::{cell::RefCell, sync::Arc};

new_key_type! { pub(crate) struct NodeId; }

pub(crate) struct Arena;

type ArenaMap = SlotMap<NodeId, Box<dyn Any + Send + Sync>>;

#[cfg(not(feature = "sandboxed-arenas"))]
static MAP: OnceLock<RwLock<ArenaMap>> = OnceLock::new();
#[cfg(feature = "sandboxed-arenas")]
thread_local! {
    pub(crate) static MAP: RefCell<Option<Arc<RwLock<ArenaMap>>>> = RefCell::new(None);
}

impl Arena {
    #[inline(always)]
    pub fn enter_new() {
        #[cfg(feature = "sandboxed-arenas")]
        MAP.with_borrow_mut(|arena| {
            *arena =
                Some(Arc::new(RwLock::new(SlotMap::with_capacity_and_key(32))))
        })
    }

    #[track_caller]
    pub fn with<U>(fun: impl FnOnce(&ArenaMap) -> U) -> U {
        #[cfg(not(feature = "sandboxed-arenas"))]
        {
            fun(&MAP.get_or_init(Default::default).read().or_poisoned())
        }
        #[cfg(feature = "sandboxed-arenas")]
        {
            MAP.with_borrow(|arena| {
                fun(&arena
                    .as_ref()
                    .unwrap_or_else(|| {
                        panic!(
                            "at {}, the `sandboxed-arenas` feature is active, \
                             but no Arena is active",
                            std::panic::Location::caller()
                        )
                    })
                    .read()
                    .or_poisoned())
            })
        }
    }

    #[track_caller]
    pub fn with_mut<U>(fun: impl FnOnce(&mut ArenaMap) -> U) -> U {
        #[cfg(not(feature = "sandboxed-arenas"))]
        {
            fun(&mut MAP.get_or_init(Default::default).write().or_poisoned())
        }
        #[cfg(feature = "sandboxed-arenas")]
        {
            let caller = std::panic::Location::caller();
            MAP.with_borrow(|arena| {
                fun(&mut arena
                    .as_ref()
                    .unwrap_or_else(|| {
                        panic!(
                            "at {}, the `sandboxed-arenas` feature is active, \
                             but no Arena is active",
                            caller
                        )
                    })
                    .write()
                    .or_poisoned())
            })
        }
    }
}

#[cfg(feature = "sandboxed-arenas")]
pub mod sandboxed {
    use super::{Arena, ArenaMap, MAP};
    use futures::Stream;
    use pin_project_lite::pin_project;
    use std::{
        future::Future,
        mem,
        pin::Pin,
        sync::{Arc, RwLock},
        task::{Context, Poll},
    };

    impl Arena {
        fn set(new_arena: Arc<RwLock<ArenaMap>>) -> UnsetArenaOnDrop {
            MAP.with_borrow_mut(|arena| {
                UnsetArenaOnDrop(mem::replace(arena, Some(new_arena)))
            })
        }
    }

    pin_project! {
        pub struct Sandboxed<T> {
            arena: Arc<RwLock<ArenaMap>>,
            #[pin]
            inner: T,
        }
    }

    impl<T> Sandboxed<T> {
        pub fn new(inner: T) -> Self {
            let arena = MAP.with_borrow(|current| {
                Arc::clone(current.as_ref().expect(
                    "the `sandboxed-arenas` feature is active, but no Arena \
                     is active",
                ))
            });
            Self { arena, inner }
        }
    }

    impl<Fut> Future for Sandboxed<Fut>
    where
        Fut: Future,
    {
        type Output = Fut::Output;

        fn poll(
            self: Pin<&mut Self>,
            cx: &mut Context<'_>,
        ) -> Poll<Self::Output> {
            let unset = Arena::set(Arc::clone(&self.arena));
            let this = self.project();
            let res = this.inner.poll(cx);
            drop(unset);
            res
        }
    }

    impl<T> Stream for Sandboxed<T>
    where
        T: Stream,
    {
        type Item = T::Item;

        fn poll_next(
            self: Pin<&mut Self>,
            cx: &mut Context<'_>,
        ) -> Poll<Option<Self::Item>> {
            let unset = Arena::set(Arc::clone(&self.arena));
            let this = self.project();
            let res = this.inner.poll_next(cx);
            drop(unset);
            res
        }
    }

    #[derive(Debug)]
    struct UnsetArenaOnDrop(Option<Arc<RwLock<ArenaMap>>>);

    impl Drop for UnsetArenaOnDrop {
        fn drop(&mut self) {
            if let Some(inner) = self.0.take() {
                MAP.with_borrow_mut(|current_map| *current_map = Some(inner));
            }
        }
    }
}
