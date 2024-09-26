use or_poisoned::OrPoisoned;
use slotmap::{new_key_type, SlotMap};
#[cfg(feature = "sandboxed-arenas")]
use std::cell::RefCell;
#[cfg(not(feature = "sandboxed-arenas"))]
use std::sync::OnceLock;
#[cfg(feature = "sandboxed-arenas")]
use std::sync::Weak;
use std::{
    any::Any,
    hash::Hash,
    sync::{Arc, RwLock},
};

new_key_type! {
    /// Unique identifier for an item stored in the arena.
    pub struct NodeId;
}

pub struct Arena;

pub type ArenaMap = SlotMap<NodeId, Box<dyn Any + Send + Sync>>;

#[cfg(not(feature = "sandboxed-arenas"))]
static MAP: OnceLock<RwLock<ArenaMap>> = OnceLock::new();
#[cfg(feature = "sandboxed-arenas")]
thread_local! {
    pub(crate) static MAP: RefCell<Option<Weak<RwLock<ArenaMap>>>> = RefCell::new(Some(Default::default()));
}

impl Arena {
    #[inline(always)]
    #[allow(unused)]
    pub fn set(arena: &Arc<RwLock<ArenaMap>>) {
        #[cfg(feature = "sandboxed-arenas")]
        {
            let new_arena = Arc::downgrade(arena);
            MAP.with_borrow_mut(|arena| {
                *arena = Some(new_arena);
            })
        }
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
                    .and_then(Weak::upgrade)
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
                    .and_then(Weak::upgrade)
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
        pin::Pin,
        sync::{Arc, RwLock, Weak},
        task::{Context, Poll},
    };

    pin_project! {
        /// A [`Future`] that restores its associated arena as the current arena whenever it is
        /// polled.
        ///
        /// Sandboxed arenas are used to ensure that data created in response to e.g., different
        /// HTTP requests can be handled separately, while providing stable identifiers for their
        /// stored values. Wrapping a `Future` in `Sandboxed` ensures that it will always use the
        /// same arena that it was created under.
        pub struct Sandboxed<T> {
            arena: Option<Arc<RwLock<ArenaMap>>>,
            #[pin]
            inner: T,
        }
    }

    impl<T> Sandboxed<T> {
        /// Wraps the given [`Future`], ensuring that any [`ArenaItem`] created while it is being
        /// polled will be associated with the same arena that was active when this was called.
        pub fn new(inner: T) -> Self {
            let arena = MAP.with_borrow(|n| n.as_ref().and_then(Weak::upgrade));
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
            if let Some(arena) = self.arena.as_ref() {
                Arena::set(arena);
            }
            let this = self.project();
            this.inner.poll(cx)
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
            if let Some(arena) = self.arena.as_ref() {
                Arena::set(arena);
            }
            let this = self.project();
            this.inner.poll_next(cx)
        }
    }
}
