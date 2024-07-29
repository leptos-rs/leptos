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

    /// A version of [`Effect::new`] that only listens to any dependency
    /// that is accessed inside `dependency_fn`.
    ///
    /// The return value of `dependency_fn` is passed into `handler` as an argument together with the previous value.
    /// Additionally, the last return value of `handler` is provided as a third argument, as is done in [`Effect::new`].
    ///
    /// ## Usage
    ///
    /// ```
    /// # use reactive_graph::effect::Effect;
    /// # use reactive_graph::traits::*;
    /// # use reactive_graph::signal::signal;
    /// # tokio_test::block_on(async move {
    /// # tokio::task::LocalSet::new().run_until(async move {
    /// #
    /// let (num, set_num) = signal(0);
    ///
    /// let effect = Effect::watch(
    ///     move || num.get(),
    ///     move |num, prev_num, _| {
    ///         // log::debug!("Number: {}; Prev: {:?}", num, prev_num);
    ///     },
    ///     false,
    /// );
    ///
    /// set_num.set(1); // > "Number: 1; Prev: Some(0)"
    ///
    /// effect.stop(); // stop watching
    ///
    /// set_num.set(2); // (nothing happens)
    /// # });
    /// # });
    /// ```
    ///
    /// The callback itself doesn't track any signal that is accessed within it.
    ///
    /// ```
    /// # use reactive_graph::effect::Effect;
    /// # use reactive_graph::traits::*;
    /// # use reactive_graph::signal::signal;
    /// # tokio_test::block_on(async move {
    /// # tokio::task::LocalSet::new().run_until(async move {
    /// #
    /// let (num, set_num) = signal(0);
    /// let (cb_num, set_cb_num) = signal(0);
    ///
    /// Effect::watch(
    ///     move || num.get(),
    ///     move |num, _, _| {
    ///         // log::debug!("Number: {}; Cb: {}", num, cb_num.get());
    ///     },
    ///     false,
    /// );
    ///
    /// set_num.set(1); // > "Number: 1; Cb: 0"
    ///
    /// set_cb_num.set(1); // (nothing happens)
    ///
    /// set_num.set(2); // > "Number: 2; Cb: 1"
    /// # });
    /// # });
    /// ```
    ///
    /// ## Immediate
    ///
    /// If the final parameter `immediate` is true, the `callback` will run immediately.
    /// If it's `false`, the `callback` will run only after
    /// the first change is detected of any signal that is accessed in `deps`.
    ///
    /// ```
    /// # use reactive_graph::effect::Effect;
    /// # use reactive_graph::traits::*;
    /// # use reactive_graph::signal::signal;
    /// # tokio_test::block_on(async move {
    /// # tokio::task::LocalSet::new().run_until(async move {
    /// #
    /// let (num, set_num) = signal(0);
    ///
    /// Effect::watch(
    ///     move || num.get(),
    ///     move |num, prev_num, _| {
    ///         // log::debug!("Number: {}; Prev: {:?}", num, prev_num);
    ///     },
    ///     true,
    /// ); // > "Number: 0; Prev: None"
    ///
    /// set_num.set(1); // > "Number: 1; Prev: Some(0)"
    /// # });
    /// # });
    /// ```
    pub fn watch<D, T>(
        mut dependency_fn: impl FnMut() -> D + 'static,
        mut handler: impl FnMut(&D, Option<&D>, Option<T>) -> T + 'static,
        immediate: bool,
    ) -> Self
    where
        D: 'static,
        T: 'static,
    {
        let (mut rx, owner, inner) = effect_base();
        let mut first_run = true;
        let dep_value = Arc::new(RwLock::new(None::<D>));
        let watch_value = Arc::new(RwLock::new(None::<T>));

        if cfg!(feature = "effects") {
            Executor::spawn_local({
                let dep_value = Arc::clone(&dep_value);
                let watch_value = Arc::clone(&watch_value);
                let subscriber = inner.to_any_subscriber();

                async move {
                    while rx.next().await.is_some() {
                        if first_run
                            || subscriber.with_observer(|| {
                                subscriber.update_if_necessary()
                            })
                        {
                            subscriber.clear_sources(&subscriber);

                            let old_dep_value = mem::take(
                                &mut *dep_value.write().or_poisoned(),
                            );
                            let new_dep_value = owner.with_cleanup(|| {
                                subscriber.with_observer(|| dependency_fn())
                            });

                            let old_watch_value = mem::take(
                                &mut *watch_value.write().or_poisoned(),
                            );

                            if immediate || !first_run {
                                let new_watch_value = handler(
                                    &new_dep_value,
                                    old_dep_value.as_ref(),
                                    old_watch_value,
                                );

                                *watch_value.write().or_poisoned() =
                                    Some(new_watch_value);
                            }

                            *dep_value.write().or_poisoned() =
                                Some(new_dep_value);

                            first_run = false;
                        }
                    }
                }
            });
        }

        Self {
            inner: StoredValue::new_with_storage(Some(inner)),
        }
    }

    /// This is to [`Effect::watch`] what [`Effect::new_sync`] is to [`Effect::new`].
    pub fn watch_sync<D, T>(
        mut dependency_fn: impl FnMut() -> D + Send + Sync + 'static,
        mut handler: impl FnMut(&D, Option<&D>, Option<T>) -> T
            + Send
            + Sync
            + 'static,
        immediate: bool,
    ) -> Self
    where
        D: Send + Sync + 'static,
        T: Send + Sync + 'static,
    {
        let (mut rx, owner, inner) = effect_base();
        let mut first_run = true;
        let dep_value = Arc::new(RwLock::new(None::<D>));
        let watch_value = Arc::new(RwLock::new(None::<T>));

        if cfg!(feature = "effects") {
            Executor::spawn({
                let dep_value = Arc::clone(&dep_value);
                let watch_value = Arc::clone(&watch_value);
                let subscriber = inner.to_any_subscriber();

                async move {
                    while rx.next().await.is_some() {
                        if first_run
                            || subscriber.with_observer(|| {
                                subscriber.update_if_necessary()
                            })
                        {
                            subscriber.clear_sources(&subscriber);

                            let old_dep_value = mem::take(
                                &mut *dep_value.write().or_poisoned(),
                            );
                            let new_dep_value = owner.with_cleanup(|| {
                                subscriber.with_observer(|| dependency_fn())
                            });

                            let old_watch_value = mem::take(
                                &mut *watch_value.write().or_poisoned(),
                            );

                            if immediate || !first_run {
                                let new_watch_value = handler(
                                    &new_dep_value,
                                    old_dep_value.as_ref(),
                                    old_watch_value,
                                );

                                *watch_value.write().or_poisoned() =
                                    Some(new_watch_value);
                            }

                            *dep_value.write().or_poisoned() =
                                Some(new_dep_value);

                            first_run = false;
                        }
                    }
                }
            });
        }

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
