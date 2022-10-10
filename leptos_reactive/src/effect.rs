use crate::{Runtime, Scope, ScopeId, Source, Subscriber};
use serde::{Deserialize, Serialize};
use std::{any::type_name, cell::RefCell, collections::HashSet, fmt::Debug, marker::PhantomData};

/// Effects run a certain chunk of code whenever the signals they depend on change.
/// `create_effect` immediately runs the given function once, tracks its dependence
/// on any signal values read within it, and reruns the function whenever the value
/// of a dependency changes.
///
/// Effects are intended to run *side-effects* of the system, not to synchronize state
/// *within* the system. In other words: don't write to signals within effects.
/// (If you need to define a signal that depends on the value of other signals, use a
/// derived signal or [create_memo]).
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
/// # create_scope(|cx| {
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
/// # assert_eq!(b(), 2);
/// # }).dispose();
/// ```
pub fn create_effect<T>(cx: Scope, f: impl FnMut(Option<T>) -> T + 'static)
where
    T: Debug + 'static,
{
    cx.create_eff(false, f)
}

/// Creates an effect; unlike effects created by [create_effect], isomorphic effects will run on
/// the server as well as the client.
/// ```
/// # use leptos_reactive::*;
/// # use log::*;
/// # create_scope(|cx| {
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
pub fn create_isomorphic_effect<T>(cx: Scope, f: impl FnMut(Option<T>) -> T + 'static)
where
    T: Debug + 'static,
{
    cx.create_isomorphic_eff(f)
}

#[doc(hidden)]
pub fn create_render_effect<T>(cx: Scope, f: impl FnMut(Option<T>) -> T + 'static)
where
    T: Debug + 'static,
{
    cx.create_eff(true, f)
}

impl Scope {
    #[cfg(not(feature = "ssr"))]
    pub(crate) fn create_eff<T>(self, render_effect: bool, f: impl FnMut(Option<T>) -> T + 'static)
    where
        T: Debug + 'static,
    {
        let state = EffectState::new(self.runtime, render_effect, f);

        let id = self.push_effect(state);

        self.runtime
            .any_effect((self.id, id), |effect| effect.run((self.id, id)));
    }

    // Simply don't run effects on the server at all
    #[cfg(feature = "ssr")]
    pub(crate) fn create_eff<T>(
        self,
        _render_effect: bool,
        _f: impl FnMut(Option<T>) -> T + 'static,
    ) where
        T: Debug + 'static,
    {
    }

    pub(crate) fn create_isomorphic_eff<T>(self, f: impl FnMut(Option<T>) -> T + 'static)
    where
        T: Debug + 'static,
    {
        let state = EffectState::new(self.runtime, false, f);

        let id = self.push_effect(state);

        self.runtime
            .any_effect((self.id, id), |effect| effect.run((self.id, id)));
    }
}

#[doc(hidden)]
#[derive(Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Effect<T>
where
    T: 'static,
{
    pub(crate) scope: ScopeId,
    pub(crate) id: EffectId,
    pub(crate) ty: PhantomData<T>,
}

impl<T> Clone for Effect<T> {
    fn clone(&self) -> Self {
        Self {
            scope: self.scope,
            id: self.id,
            ty: PhantomData,
        }
    }
}

impl<T> Copy for Effect<T> {}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct EffectId(pub(crate) usize);

pub(crate) struct EffectState<T> {
    runtime: &'static Runtime,
    render_effect: bool,
    f: Box<RefCell<dyn FnMut(Option<T>) -> T>>,
    value: RefCell<Option<T>>,
    sources: RefCell<HashSet<Source>>,
}

impl<T> EffectState<T> {
    pub fn new(
        runtime: &'static Runtime,
        render_effect: bool,
        f: impl FnMut(Option<T>) -> T + 'static,
    ) -> Self {
        Self {
            runtime,
            render_effect,
            f: Box::new(RefCell::new(f)),
            value: Default::default(),
            sources: Default::default(),
        }
    }
}

impl<T> EffectState<T> {
    pub(crate) fn add_source(&self, source: Source) {
        self.sources.borrow_mut().insert(source);
    }

    fn cleanup(&self, id: (ScopeId, EffectId)) {
        for source in self.sources.borrow().iter() {
            source.unsubscribe(self.runtime, Subscriber(id))
        }
    }
}

impl<T> Debug for EffectState<T>
where
    T: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EffectState")
            .field(
                "f",
                &format!(
                    "FnMut<Option<&{}>> -> {}",
                    type_name::<T>(),
                    type_name::<T>()
                ),
            )
            //.field("value", &self.value)
            //.field("sources", &self.sources)
            .finish()
    }
}

pub(crate) trait AnyEffect: Debug {
    fn run(&self, id: (ScopeId, EffectId));

    fn clear_dependencies(&self);

    fn subscribe_to(&self, source: Source);
}

impl<T> AnyEffect for EffectState<T>
where
    T: Debug + 'static,
{
    fn run(&self, id: (ScopeId, EffectId)) {
        // clear previous dependencies
        // at this point, Effect dependencies have been added to Signal
        // and any Signal changes will call Effect dependency automatically
        self.cleanup(id);

        // add it to the Scope stack, which means any signals called
        // in the effect fn immediately below will add this Effect as a dependency
        self.runtime.push_stack(Subscriber(id));

        // actually run the effect
        #[cfg(feature = "transition")]
        if let Some(transition) = self.runtime.running_transition() && self.render_effect {
            transition.effects.borrow_mut().push(id);
        } else {
            let curr = { self.value.borrow_mut().take() };
            let v = { (self.f.borrow_mut())(curr) };
            *self.value.borrow_mut() = Some(v);
        }

        #[cfg(not(feature = "transition"))]
        {
            let curr = { self.value.borrow_mut().take() };
            let v = { (self.f.borrow_mut())(curr) };
            *self.value.borrow_mut() = Some(v);
        }

        // pop it back off the stack
        self.runtime.pop_stack();
    }

    fn clear_dependencies(&self) {
        self.sources.borrow_mut().clear();
    }

    fn subscribe_to(&self, source: Source) {
        self.add_source(source);
    }
}
