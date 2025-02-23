use crate::{
    graph::{AnySubscriber, ReactiveNode, ToAnySubscriber},
    owner::{ArenaItem, LocalStorage, Storage, SyncStorage},
    traits::Dispose,
};
use std::sync::{Arc, Mutex, RwLock};

/// Effects run a certain chunk of code whenever the signals they depend on change.
///
/// The effect runs on creation and again as soon as any tracked signal changes.
///
/// NOTE: you probably want use [`Effect`](super::Effect) instead.
/// This is for the few cases where it's important to execute effects immediately and in order.
///
/// Effects are intended to run *side-effects* of the system, not to synchronize state
/// *within* the system. In other words: In most cases, you usually should not write to
/// signals inside effects. (If you need to define a signal that depends on the value of
/// other signals, use a derived signal or a [`Memo`](crate::computed::Memo)).
///
/// You can provide an effect function without parameters or one with one parameter.
/// If you provide such a parameter, the effect function is called with an argument containing
/// whatever value it returned the last time it ran. On the initial run, this is `None`.
///
/// Effects stop running when their reactive [`Owner`](crate::owner::Owner) is disposed.
///
/// ## Example
///
/// ```
/// # use reactive_graph::computed::*;
/// # use reactive_graph::signal::*; let owner = reactive_graph::owner::Owner::new(); owner.set();
/// # use reactive_graph::prelude::*;
/// # use reactive_graph::effect::immediateEffect;
/// # use reactive_graph::owner::ArenaItem;
/// # let owner = reactive_graph::owner::Owner::new(); owner.set();
/// let a = RwSignal::new(0);
/// let b = RwSignal::new(0);
///
/// // ✅ use effects to interact between reactive state and the outside world
/// ImmediateEffect::new(move || {
///   // on the next “tick” prints "Value: 0" and subscribes to `a`
///   println!("Value: {}", a.get());
/// });
///
/// // The effect runs immediately and subscribes to `a`, in the process it prints "Value: 0"
/// # assert_eq!(a.get(), 0);
/// a.set(1);
/// # assert_eq!(a.get(), 1);
/// // ✅ because it's subscribed to `a`, the effect reruns and prints "Value: 1"
///
/// // ❌ don't use effects to synchronize state within the reactive system
/// Effect::new(move || {
///   // this technically works but can cause unnecessary runs
///   // and easily lead to problems like infinite loops
///   b.set(a.get() + 1);
/// });
/// ```
/// ## Web-Specific Notes
///
/// 1. **Scheduling**: Effects run immediately, as soon as any tracked signal changes.
/// 2. By default, effects do not run unless the `effects` feature is enabled. If you are using
///    this with a web framework, this generally means that effects **do not run on the server**.
///    and you can call browser-specific APIs within the effect function without causing issues.
///    If you need an effect to run on the server, use [`ImmediateEffect::new_isomorphic`].
#[derive(Debug, Clone, Copy)]
pub struct ImmediateEffect<S> {
    inner: Option<ArenaItem<StoredEffect, S>>,
}

type StoredEffect = Option<Arc<RwLock<inner::EffectInner>>>;

impl<S> Dispose for ImmediateEffect<S> {
    fn dispose(self) {
        if let Some(inner) = self.inner {
            inner.dispose()
        }
    }
}

impl ImmediateEffect<LocalStorage> {
    /// Creates a new effect which runs immediately, then again as soon as any tracked signal changes.
    ///
    /// NOTE: this requires a `Fn` function because it might recurse.
    /// Use [Self::new_mut] to pass a `FnMut` function, it'll panic on recursion.
    pub fn new<T>(fun: impl Fn(Option<T>) -> T + Send + Sync + 'static) -> Self
    where
        T: Send + Sync + 'static,
    {
        if !cfg!(feature = "effects") {
            return Self { inner: None };
        }

        let inner = inner::EffectInner::new(fun);

        inner.update_if_necessary();

        Self {
            inner: Some(ArenaItem::new_with_storage(Some(inner))),
        }
    }
    /// Creates a new effect which runs immediately, then again as soon as any tracked signal changes.
    ///
    /// # Panics
    /// Panics on recursion. Also see [Self::new]
    pub fn new_mut<T>(
        fun: impl FnMut(Option<T>) -> T + Send + Sync + 'static,
    ) -> Self
    where
        T: Send + Sync + 'static,
    {
        const MSG: &str = "The effect recursed or its function panicked.";
        let fun = Mutex::new(fun);
        Self::new(move |v| fun.try_lock().expect(MSG)(v))
    }
}

impl ImmediateEffect<SyncStorage> {
    /// Creates a new effect which runs immediately, then again as soon as any tracked signal changes.
    pub fn new_sync<T>(
        fun: impl Fn(Option<T>) -> T + Send + Sync + 'static,
    ) -> Self
    where
        T: Send + Sync + 'static,
    {
        if !cfg!(feature = "effects") {
            return Self { inner: None };
        }

        Self::new_isomorphic(fun)
    }

    /// Creates a new effect which runs immediately, then again as soon as any tracked signal changes.
    ///
    /// This will run whether the `effects` feature is enabled or not.
    pub fn new_isomorphic<T>(
        fun: impl Fn(Option<T>) -> T + Send + Sync + 'static,
    ) -> Self
    where
        T: Send + Sync + 'static,
    {
        let inner = inner::EffectInner::new(fun);

        inner.update_if_necessary();

        Self {
            inner: Some(ArenaItem::new_with_storage(Some(inner))),
        }
    }
}

impl<S> ToAnySubscriber for ImmediateEffect<S>
where
    S: Storage<StoredEffect>,
{
    fn to_any_subscriber(&self) -> AnySubscriber {
        const MSG: &str = "tried to set effect that has been stopped";
        let inner = self.inner.as_ref().expect(MSG);
        inner
            .try_with_value(|inner| Some(inner.as_ref()?.to_any_subscriber()))
            .flatten()
            .expect(MSG)
    }
}

mod inner {
    use crate::{
        graph::{
            AnySource, AnySubscriber, ReactiveNode, ReactiveNodeState,
            SourceSet, Subscriber, ToAnySubscriber, WithObserver,
        },
        owner::Owner,
    };
    use or_poisoned::OrPoisoned;
    use std::sync::{Arc, Mutex, RwLock, Weak};

    /// Handles subscription logic for effects.
    pub(super) struct EffectInner {
        owner: Owner,
        state: ReactiveNodeState,
        fun: Arc<dyn Fn() + Send + Sync>,
        sources: SourceSet,
        any_subscriber: AnySubscriber,
    }

    impl EffectInner {
        pub fn new<T>(
            fun: impl Fn(Option<T>) -> T + Send + Sync + 'static,
        ) -> Arc<RwLock<EffectInner>>
        where
            T: Send + Sync + 'static,
        {
            let owner = Owner::new();

            let fun = {
                let value = Mutex::new(None);
                move || {
                    let old = value.lock().unwrap().take();
                    let new = fun(old);
                    value.lock().unwrap().replace(new);
                }
            };

            Arc::new_cyclic(|weak| {
                let any_subscriber = AnySubscriber(
                    weak.as_ptr() as usize,
                    Weak::clone(weak) as Weak<dyn Subscriber + Send + Sync>,
                );

                RwLock::new(EffectInner {
                    owner,
                    state: ReactiveNodeState::Dirty,
                    fun: Arc::new(fun),
                    sources: SourceSet::new(),
                    any_subscriber,
                })
            })
        }
    }

    impl ToAnySubscriber for Arc<RwLock<EffectInner>> {
        fn to_any_subscriber(&self) -> AnySubscriber {
            AnySubscriber(
                Arc::as_ptr(self) as usize,
                Arc::downgrade(self) as Weak<dyn Subscriber + Send + Sync>,
            )
        }
    }

    impl ReactiveNode for RwLock<EffectInner> {
        fn mark_subscribers_check(&self) {}

        fn update_if_necessary(&self) -> bool {
            let state = {
                let guard = self.read().or_poisoned();

                if guard.owner.paused() {
                    return false;
                }

                guard.state
            };

            let needs_update = match state {
                ReactiveNodeState::Clean => false,
                ReactiveNodeState::Check => {
                    let sources = self.read().or_poisoned().sources.clone();
                    sources
                        .into_iter()
                        .any(|source| source.update_if_necessary())
                }
                ReactiveNodeState::Dirty => true,
            };

            if needs_update {
                let guard = self.read().or_poisoned();

                let owner = guard.owner.clone();
                let any_subscriber = guard.any_subscriber.clone();
                let fun = guard.fun.clone();

                drop(guard);

                any_subscriber.clear_sources(&any_subscriber);

                owner.with_cleanup(|| any_subscriber.with_observer(|| fun()));

                self.write().or_poisoned().state = ReactiveNodeState::Clean;
            }

            needs_update
        }

        fn mark_check(&self) {
            self.write().or_poisoned().state = ReactiveNodeState::Check;
            self.update_if_necessary();
        }

        fn mark_dirty(&self) {
            self.write().or_poisoned().state = ReactiveNodeState::Dirty;
            self.update_if_necessary();
        }
    }

    impl Subscriber for RwLock<EffectInner> {
        fn add_source(&self, source: AnySource) {
            self.write().or_poisoned().sources.insert(source);
        }

        fn clear_sources(&self, subscriber: &AnySubscriber) {
            self.write().or_poisoned().sources.clear_sources(subscriber);
        }
    }

    impl std::fmt::Debug for EffectInner {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("EffectInner")
                .field("owner", &self.owner)
                .field("state", &self.state)
                .field("sources", &self.sources)
                .field("any_subscriber", &self.any_subscriber)
                .finish()
        }
    }
}
