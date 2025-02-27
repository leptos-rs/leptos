use std::{
    panic::Location,
    sync::{Arc, Mutex, RwLock},
};

use or_poisoned::OrPoisoned;

use crate::{
    graph::{AnySubscriber, ReactiveNode, ToAnySubscriber},
    owner::{ArenaItem, LocalStorage, Storage, SyncStorage},
    traits::{DefinedAt, Dispose},
};

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
/// NOTE: since effects are executed immediately, they might recurse.
/// Under recursion only the last run to start is tracked.
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
    #[track_caller]
    pub fn new(fun: impl Fn() + Send + Sync + 'static) -> Self {
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
    #[track_caller]
    pub fn new_mut(fun: impl FnMut() + Send + Sync + 'static) -> Self {
        const MSG: &str = "The effect recursed or its function panicked.";
        let fun = Mutex::new(fun);
        Self::new(move || fun.try_lock().expect(MSG)())
    }
}

impl ImmediateEffect<SyncStorage> {
    /// Creates a new effect which runs immediately, then again as soon as any tracked signal changes.
    #[track_caller]
    pub fn new_sync(fun: impl Fn() + Send + Sync + 'static) -> Self {
        if !cfg!(feature = "effects") {
            return Self { inner: None };
        }

        Self::new_isomorphic(fun)
    }

    /// Creates a new effect which runs immediately, then again as soon as any tracked signal changes.
    ///
    /// This will run whether the `effects` feature is enabled or not.
    #[track_caller]
    pub fn new_isomorphic(fun: impl Fn() + Send + Sync + 'static) -> Self {
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

impl<S> DefinedAt for ImmediateEffect<S>
where
    S: Storage<StoredEffect>,
{
    fn defined_at(&self) -> Option<&'static Location<'static>> {
        self.inner
            .as_ref()?
            .try_with_value(|inner| {
                inner.as_ref()?.read().or_poisoned().defined_at()
            })
            .flatten()
    }
}

mod inner {
    use crate::{
        graph::{
            AnySource, AnySubscriber, ReactiveNode, ReactiveNodeState,
            SourceSet, Subscriber, ToAnySubscriber, WithObserver,
        },
        log_warning,
        owner::Owner,
        traits::DefinedAt,
    };
    use or_poisoned::OrPoisoned;
    use std::{
        panic::Location,
        sync::{Arc, RwLock, Weak},
        thread::{self, ThreadId},
    };

    /// Handles subscription logic for effects.
    ///
    /// To handle parallelism and recursion we assign ordered (1..) ids to each run.
    /// We only keep the sources tracked by the run with the highest id (the last one).
    ///
    /// We do this by:
    /// - Clearing the sources before every run, so the last one clears anything before it.
    /// - We stop tracking sources after the last run has completed.
    ///   (A parent run will start before and end after a recursive child run.)
    /// - To handle parallelism with the last run, we only allow sources to be added by its thread.
    pub(super) struct EffectInner {
        #[cfg(any(debug_assertions, leptos_debuginfo))]
        defined_at: &'static Location<'static>,
        owner: Owner,
        state: ReactiveNodeState,
        /// The number of effect runs in this 'batch'.
        /// Cleared when no runs are *ongoing* anymore.
        /// Used to assign ordered ids to each run, and to know when we can clear these values.
        run_count_start: usize,
        /// The number of effect runs that have completed in the current 'batch'.
        /// Cleared when no runs are *ongoing* anymore.
        /// Used to know when we can clear these values.
        run_done_count: usize,
        /// Given ordered ids (1..), the run with the highest id that has completed in this 'batch'.
        /// Cleared when no runs are *ongoing* anymore.
        /// Used to know whether the current run is the latest one.
        run_done_max: usize,
        /// The [ThreadId] of the run with the highest id.
        /// Used to prevent over-subscribing during parallel execution with the last run.
        last_run_thread_id: ThreadId,
        fun: Arc<dyn Fn() + Send + Sync>,
        sources: SourceSet,
        any_subscriber: AnySubscriber,
    }

    impl EffectInner {
        #[track_caller]
        pub fn new(
            fun: impl Fn() + Send + Sync + 'static,
        ) -> Arc<RwLock<EffectInner>> {
            let owner = Owner::new();

            Arc::new_cyclic(|weak| {
                let any_subscriber = AnySubscriber(
                    weak.as_ptr() as usize,
                    Weak::clone(weak) as Weak<dyn Subscriber + Send + Sync>,
                );

                RwLock::new(EffectInner {
                    #[cfg(any(debug_assertions, leptos_debuginfo))]
                    defined_at: Location::caller(),
                    owner,
                    state: ReactiveNodeState::Dirty,
                    run_count_start: 0,
                    run_done_count: 0,
                    run_done_max: 0,
                    last_run_thread_id: thread::current().id(),
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
                let mut guard = self.write().or_poisoned();

                let owner = guard.owner.clone();
                let any_subscriber = guard.any_subscriber.clone();
                let fun = guard.fun.clone();

                // New run has started.
                guard.run_count_start += 1;
                // We get a value for this run, the highest value will be what we keep the sources from.
                let recursion_count = guard.run_count_start;
                // We clear the sources before running the effect.
                // Note that this is tied to the ordering of the initial write lock acquisition
                // to ensure the last run is also the last to clear them.
                guard.sources.clear_sources(&any_subscriber);
                // Only this thread will be able to subscribe.
                guard.last_run_thread_id = thread::current().id();

                if recursion_count > 2 {
                    warn_excessive_recursion(&guard);
                }

                drop(guard);

                // We execute the effect.
                // Note that *this could happen in parallel across threads*.
                owner.with_cleanup(|| any_subscriber.with_observer(|| fun()));

                let mut guard = self.write().or_poisoned();

                // This run has completed.
                guard.run_done_count += 1;

                // We update the done count.
                // Sources will only be added if recursion_done_max < recursion_count_start.
                // (Meaning the last run is not done yet.)
                guard.run_done_max =
                    Ord::max(recursion_count, guard.run_done_max);

                // The same amount of runs has started and completed,
                // so we can clear everything up for next time.
                if guard.run_count_start == guard.run_done_count {
                    guard.run_count_start = 0;
                    guard.run_done_count = 0;
                    guard.run_done_max = 0;
                    // Can be left unchanged, it'll be set again next time.
                    // guard.last_run_thread_id = thread::current().id();
                }

                guard.state = ReactiveNodeState::Clean;
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
            let mut guard = self.write().or_poisoned();
            if guard.run_done_max < guard.run_count_start
                && guard.last_run_thread_id == thread::current().id()
            {
                guard.sources.insert(source);
            }
        }

        fn clear_sources(&self, subscriber: &AnySubscriber) {
            self.write().or_poisoned().sources.clear_sources(subscriber);
        }
    }

    impl DefinedAt for EffectInner {
        fn defined_at(&self) -> Option<&'static Location<'static>> {
            #[cfg(any(debug_assertions, leptos_debuginfo))]
            {
                Some(self.defined_at)
            }
            #[cfg(not(any(debug_assertions, leptos_debuginfo)))]
            {
                None
            }
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

    fn warn_excessive_recursion(effect: &EffectInner) {
        const MSG: &str = "ImmediateEffect recursed more than once.";
        match effect.defined_at() {
            Some(defined_at) => {
                log_warning(format_args!("{MSG} Defined at: {}", defined_at));
            }
            None => {
                log_warning(format_args!("{MSG}"));
            }
        }
    }
}
