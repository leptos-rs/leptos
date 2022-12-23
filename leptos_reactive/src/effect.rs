use crate::runtime::{with_runtime, RuntimeId};
use crate::{debug_warn, Runtime, Scope, ScopeProperty};
use cfg_if::cfg_if;
use std::cell::RefCell;
use std::fmt::Debug;

/// Effects run a certain chunk of code whenever the signals they depend on change.
/// `create_effect` immediately runs the given function once, tracks its dependence
/// on any signal values read within it, and reruns the function whenever the value
/// of a dependency changes.
///
/// Effects are intended to run *side-effects* of the system, not to synchronize state
/// *within* the system. In other words: don't write to signals within effects.
/// (If you need to define a signal that depends on the value of other signals, use a
/// derived signal or [create_memo](crate::create_memo)).
///
/// The effect function is called with an argument containing whatever value it returned
/// the last time it ran. On the initial run, this is `None`.
///
/// By default, effects **do not run on the server**. This means you can call browser-specific
/// APIs within the effect function without causing issues. If you need an effect to run on
/// the server, use [create_isomorphic_effect].
/// ```
/// # use leptos_reactive::*;
/// # use log::*;
/// # create_scope(create_runtime(), |cx| {
/// let (a, set_a) = create_signal(cx, 0);
/// let (b, set_b) = create_signal(cx, 0);
///
/// // ✅ use effects to interact between reactive state and the outside world
/// create_effect(cx, move |_| {
///   // immediately prints "Value: 0" and subscribes to `a`
///   log::debug!("Value: {}", a());
/// });
///
/// set_a(1);
/// // ✅ because it's subscribed to `a`, the effect reruns and prints "Value: 1"
///
/// // ❌ don't use effects to synchronize state within the reactive system
/// create_effect(cx, move |_| {
///   // this technically works but can cause unnecessary re-renders
///   // and easily lead to problems like infinite loops
///   set_b(a() + 1);
/// });
/// # if !cfg!(feature = "ssr") {
/// # assert_eq!(b(), 2);
/// # }
/// # }).dispose();
/// ```
pub fn create_effect<T>(cx: Scope, f: impl Fn(Option<T>) -> T + 'static)
where
    T: 'static,
{
    cfg_if! {
        if #[cfg(not(feature = "ssr"))] {
            create_isomorphic_effect(cx, f);
        } else {
            // clear warnings
            _ = cx;
            _ = f;
        }
    }
}

/// Creates an effect; unlike effects created by [create_effect], isomorphic effects will run on
/// the server as well as the client.
/// ```
/// # use leptos_reactive::*;
/// # use log::*;
/// # create_scope(create_runtime(), |cx| {
/// let (a, set_a) = create_signal(cx, 0);
/// let (b, set_b) = create_signal(cx, 0);
///
/// // ✅ use effects to interact between reactive state and the outside world
/// create_isomorphic_effect(cx, move |_| {
///   // immediately prints "Value: 0" and subscribes to `a`
///   log::debug!("Value: {}", a());
/// });
///
/// set_a(1);
/// // ✅ because it's subscribed to `a`, the effect reruns and prints "Value: 1"
///
/// // ❌ don't use effects to synchronize state within the reactive system
/// create_isomorphic_effect(cx, move |_| {
///   // this technically works but can cause unnecessary re-renders
///   // and easily lead to problems like infinite loops
///   set_b(a() + 1);
/// });
/// # assert_eq!(b(), 2);
/// # }).dispose();
pub fn create_isomorphic_effect<T>(cx: Scope, f: impl Fn(Option<T>) -> T + 'static)
where
    T: 'static,
{
    let e = cx.runtime.create_effect(f);
    cx.with_scope_property(|prop| prop.push(ScopeProperty::Effect(e)))
}

#[doc(hidden)]
pub fn create_render_effect<T>(cx: Scope, f: impl Fn(Option<T>) -> T + 'static)
where
    T: 'static,
{
    create_effect(cx, f);
}

slotmap::new_key_type! {
    /// Unique ID assigned to an [Effect](crate::Effect).
    pub(crate) struct EffectId;
}

pub(crate) struct Effect<T, F>
where
    T: 'static,
    F: Fn(Option<T>) -> T,
{
    pub(crate) f: F,
    pub(crate) value: RefCell<Option<T>>,
}

pub(crate) trait AnyEffect {
    fn run(&self, id: EffectId, runtime: RuntimeId);
}

impl<T, F> AnyEffect for Effect<T, F>
where
    T: 'static,
    F: Fn(Option<T>) -> T,
{
    fn run(&self, id: EffectId, runtime: RuntimeId) {
        with_runtime(runtime, |runtime| {
            // clear previous dependencies
            id.cleanup(runtime);

            // set this as the current observer
            let prev_observer = runtime.observer.take();
            runtime.observer.set(Some(id));

            // run the effect
            let value = self.value.take();
            let new_value = (self.f)(value);
            *self.value.borrow_mut() = Some(new_value);

            // restore the previous observer
            runtime.observer.set(prev_observer);
        })
    }
}

impl EffectId {
    pub(crate) fn run<T>(&self, runtime_id: RuntimeId) {
        with_runtime(runtime_id, |runtime| {
            let effect = {
                let effects = runtime.effects.borrow();
                effects.get(*self).cloned()
            };
            if let Some(effect) = effect {
                effect.run(*self, runtime_id);
            } else {
                debug_warn!("[Effect] Trying to run an Effect that has been disposed. This is probably either a logic error in a component that creates and disposes of scopes, or a Resource resolving after its scope has been dropped without having been cleaned up.")
            }
        })
    }

    pub(crate) fn cleanup(&self, runtime: &Runtime) {
        let sources = runtime.effect_sources.borrow();
        if let Some(sources) = sources.get(*self) {
            let subs = runtime.signal_subscribers.borrow();
            for source in sources.borrow().iter() {
                if let Some(source) = subs.get(*source) {
                    source.borrow_mut().remove(self);
                }
            }
        }
    }
}
