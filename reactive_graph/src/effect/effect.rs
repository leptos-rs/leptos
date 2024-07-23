use crate::{
    channel::{channel, Receiver},
    effect::inner::EffectInner,
    graph::{
        AnySubscriber, ReactiveNode, SourceSet, Subscriber, ToAnySubscriber,
        WithObserver,
    },
    owner::{LocalStorage, Owner, StoredValue},
    traits::Dispose,
};
use any_spawner::Executor;
use futures::StreamExt;
use or_poisoned::OrPoisoned;
use std::{
    mem,
    sync::{Arc, RwLock},
};

/// Effects run a certain chunk of code whenever the signals they depend on change.
/// Creating an effect runs the given function once after any current synchronous work is done.
/// This tracks its reactive values read within it, and reruns the function whenever the value
/// of a dependency changes.
///
/// Effects are intended to run *side-effects* of the system, not to synchronize state
/// *within* the system. In other words: In most cases, you usually should not write to
/// signals inside effects. (If you need to define a signal that depends on the value of
/// other signals, use a derived signal or a [`Memo`](crate::computed::Memo)).
///
/// The effect function is called with an argument containing whatever value it returned
/// the last time it ran. On the initial run, this is `None`.
///
/// Effects stop running when their reactive [`Owner`] is disposed.
///
///
/// ## Example
///
/// ```
/// # use reactive_graph::computed::*;
/// # use reactive_graph::signal::*;
/// # use reactive_graph::prelude::*;
/// # use reactive_graph::effect::Effect;
/// # use reactive_graph::owner::StoredValue;
/// # tokio_test::block_on(async move {
/// # tokio::task::LocalSet::new().run_until(async move {
/// let a = RwSignal::new(0);
/// let b = RwSignal::new(0);
///
/// // ✅ use effects to interact between reactive state and the outside world
/// Effect::new(move |_| {
///   // on the next “tick” prints "Value: 0" and subscribes to `a`
///   println!("Value: {}", a.get());
/// });
///
/// a.set(1);
/// // ✅ because it's subscribed to `a`, the effect reruns and prints "Value: 1"
///
/// // ❌ don't use effects to synchronize state within the reactive system
/// Effect::new(move |_| {
///   // this technically works but can cause unnecessary re-renders
///   // and easily lead to problems like infinite loops
///   b.set(a.get() + 1);
/// });
/// # });
/// # });
/// ```
/// ## Web-Specific Notes
///
/// 1. **Scheduling**: Effects run after synchronous work, on the next “tick” of the reactive
///    system. This makes them suitable for “on mount” actions: they will fire immediately after
///    DOM rendering.
/// 2. By default, effects do not run unless the `effects` feature is enabled. If you are using
///    this with a web framework, this generally means that effects **do not run on the server**.
///    and you can call browser-specific APIs within the effect function without causing issues.
///    If you need an effect to run on the server, use [`Effect::new_isomorphic`].
pub struct Effect {
    inner: StoredValue<Option<Arc<RwLock<EffectInner>>>, LocalStorage>,
}

impl Dispose for Effect {
    fn dispose(self) {
        self.inner.dispose()
    }
}

fn effect_base() -> (Receiver, Owner, Arc<RwLock<EffectInner>>) {
    let (mut observer, rx) = channel();

    // spawn the effect asynchronously
    // we'll notify once so it runs on the next tick,
    // to register observed values
    observer.notify();

    let owner = Owner::new();
    let inner = Arc::new(RwLock::new(EffectInner {
        dirty: true,
        observer,
        sources: SourceSet::new(),
    }));

    (rx, owner, inner)
}

impl Effect {
    /// Stops this effect before it is disposed.
    pub fn stop(self) {
        drop(self.inner.try_update_value(|inner| inner.take()));
    }

    /// Creates a new effect, which runs once on the next “tick”, and then runs again when reactive values
    /// that are read inside it change.
    ///
    /// This spawns a task on the local thread using
    /// [`spawn_local`](any_spawner::Executor::spawn_local). For an effect that can be spawned on
    /// any thread, use [`new_sync`](Effect::new_sync).
    pub fn new<T>(mut fun: impl FnMut(Option<T>) -> T + 'static) -> Self
    where
        T: 'static,
    {
        let (mut rx, owner, inner) = effect_base();
        let value = Arc::new(RwLock::new(None::<T>));
        let mut first_run = true;

        if cfg!(feature = "effects") {
            Executor::spawn_local({
                let value = Arc::clone(&value);
                let subscriber = inner.to_any_subscriber();

                async move {
                    while rx.next().await.is_some() {
                        if first_run
                            || subscriber.with_observer(|| {
                                subscriber.update_if_necessary()
                            })
                        {
                            first_run = false;
                            subscriber.clear_sources(&subscriber);

                            let old_value =
                                mem::take(&mut *value.write().or_poisoned());
                            let new_value = owner.with_cleanup(|| {
                                subscriber.with_observer(|| fun(old_value))
                            });
                            *value.write().or_poisoned() = Some(new_value);
                        }
                    }
                }
            });
        }

        Self {
            inner: StoredValue::new_with_storage(Some(inner)),
        }
    }

    /// Creates a new effect, which runs once on the next “tick”, and then runs again when reactive values
    /// that are read inside it change.
    ///
    /// This spawns a task that can be run on any thread. For an effect that will be spawned on
    /// the current thread, use [`new`](Effect::new).
    pub fn new_sync<T>(
        mut fun: impl FnMut(Option<T>) -> T + Send + Sync + 'static,
    ) -> Self
    where
        T: Send + Sync + 'static,
    {
        let (mut rx, owner, inner) = effect_base();
        let mut first_run = true;
        let value = Arc::new(RwLock::new(None::<T>));

        if cfg!(feature = "effects") {
            Executor::spawn({
                let value = Arc::clone(&value);
                let subscriber = inner.to_any_subscriber();

                async move {
                    while rx.next().await.is_some() {
                        if first_run
                            || subscriber.with_observer(|| {
                                subscriber.update_if_necessary()
                            })
                        {
                            first_run = false;
                            subscriber.clear_sources(&subscriber);

                            let old_value =
                                mem::take(&mut *value.write().or_poisoned());
                            let new_value = owner.with_cleanup(|| {
                                subscriber.with_observer(|| fun(old_value))
                            });
                            *value.write().or_poisoned() = Some(new_value);
                        }
                    }
                }
            });
        }

        Self {
            inner: StoredValue::new_with_storage(Some(inner)),
        }
    }

    /// Creates a new effect, which runs once on the next “tick”, and then runs again when reactive values
    /// that are read inside it change.
    ///
    /// This will run whether the `effects` feature is enabled or not.
    pub fn new_isomorphic<T>(
        mut fun: impl FnMut(Option<T>) -> T + Send + Sync + 'static,
    ) -> Self
    where
        T: Send + Sync + 'static,
    {
        let (mut rx, owner, inner) = effect_base();
        let mut first_run = true;
        let value = Arc::new(RwLock::new(None::<T>));

        Executor::spawn({
            let value = Arc::clone(&value);
            let subscriber = inner.to_any_subscriber();

            async move {
                while rx.next().await.is_some() {
                    if first_run
                        || subscriber
                            .with_observer(|| subscriber.update_if_necessary())
                    {
                        first_run = false;
                        subscriber.clear_sources(&subscriber);

                        let old_value =
                            mem::take(&mut *value.write().or_poisoned());
                        let new_value = owner.with_cleanup(|| {
                            subscriber.with_observer(|| fun(old_value))
                        });
                        *value.write().or_poisoned() = Some(new_value);
                    }
                }
            }
        });
        Self {
            inner: StoredValue::new_with_storage(Some(inner)),
        }
    }
}

impl ToAnySubscriber for Effect {
    fn to_any_subscriber(&self) -> AnySubscriber {
        self.inner
            .try_with_value(|inner| {
                inner.as_ref().map(|inner| inner.to_any_subscriber())
            })
            .flatten()
            .expect("tried to subscribe to effect that has been stopped")
    }
}

/// Creates an [`Effect`].
#[inline(always)]
#[track_caller]
#[deprecated = "This function is being removed to conform to Rust \
                idioms.Please use `Effect::new()` instead."]
pub fn create_effect<T>(fun: impl FnMut(Option<T>) -> T + 'static) -> Effect
where
    T: 'static,
{
    Effect::new(fun)
}
