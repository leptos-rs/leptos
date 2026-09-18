use or_poisoned::OrPoisoned;
use slotmap::{SlotMap, new_key_type};
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
            Arena::try_with(fun).unwrap_or_else(|| {
                panic!(
                    "at {}, the `sandboxed-arenas` feature is active, but no \
                     Arena is active",
                    std::panic::Location::caller()
                )
            })
        }
    }

    #[track_caller]
    pub fn try_with<U>(fun: impl FnOnce(&ArenaMap) -> U) -> Option<U> {
        #[cfg(not(feature = "sandboxed-arenas"))]
        {
            Some(fun(&MAP.get_or_init(Default::default).read().or_poisoned()))
        }
        #[cfg(feature = "sandboxed-arenas")]
        {
            MAP.with_borrow(|arena| {
                arena
                    .as_ref()
                    .and_then(Weak::upgrade)
                    .map(|n| fun(&n.read().or_poisoned()))
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
            Arena::try_with_mut(fun).unwrap_or_else(|| {
                panic!(
                    "at {}, the `sandboxed-arenas` feature is active, but no \
                     Arena is active",
                    std::panic::Location::caller()
                )
            })
        }
    }

    #[track_caller]
    pub fn try_with_mut<U>(fun: impl FnOnce(&mut ArenaMap) -> U) -> Option<U> {
        #[cfg(not(feature = "sandboxed-arenas"))]
        {
            Some(fun(&mut MAP
                .get_or_init(Default::default)
                .write()
                .or_poisoned()))
        }
        #[cfg(feature = "sandboxed-arenas")]
        {
            MAP.with_borrow(|arena| {
                arena
                    .as_ref()
                    .and_then(Weak::upgrade)
                    .map(|n| fun(&mut n.write().or_poisoned()))
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
        /// Wraps the given [`Future`], ensuring that any [`ArenaItem`][item] created while it is
        /// being polled will be associated with the same arena that was active when this was
        /// called.
        ///
        /// [item]:[crate::owner::ArenaItem]
        #[track_caller]
        pub fn new(inner: T) -> Self {
            let arena = MAP.with_borrow(|n| n.as_ref().and_then(Weak::upgrade));
            Self { arena, inner }
        }
    }

    /// RAII guard that snapshots the current thread-local arena and restores it
    /// on drop, so polling a `Sandboxed` future does not leak its arena into the
    /// caller's context (including across `await` points and panics).
    struct RestoreArena(Option<Weak<RwLock<ArenaMap>>>);

    impl Drop for RestoreArena {
        fn drop(&mut self) {
            MAP.with_borrow_mut(|m| *m = self.0.take());
        }
    }

    impl RestoreArena {
        fn install(arena: Option<&Arc<RwLock<ArenaMap>>>) -> Self {
            let guard = MAP.with_borrow(|m| RestoreArena(m.clone()));
            if let Some(arena) = arena {
                Arena::set(arena);
            }
            guard
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
            let _restore = RestoreArena::install(self.arena.as_ref());
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
            let _restore = RestoreArena::install(self.arena.as_ref());
            let this = self.project();
            this.inner.poll_next(cx)
        }
    }
}
