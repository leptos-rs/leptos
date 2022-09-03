use crate::{Runtime, Scope, ScopeId, Source, Subscriber};
use serde::{Deserialize, Serialize};
use std::{any::type_name, cell::RefCell, collections::HashSet, fmt::Debug, marker::PhantomData};

pub fn create_render_effect<T>(cx: Scope, f: impl FnMut(Option<T>) -> T + 'static)
where
    T: Debug + 'static,
{
    cx.create_eff(true, f)
}

pub fn create_effect<T>(cx: Scope, f: impl FnMut(Option<T>) -> T + 'static)
where
    T: Debug + 'static,
{
    cx.create_eff(false, f)
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
}

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
        if let Some(transition) = self.runtime.running_transition() && self.render_effect {
            transition.effects.borrow_mut().push(id);
        } else {
            let curr = { self.value.borrow_mut().take() };
            let v = { (self.f.borrow_mut())(curr) };
            *self.value.borrow_mut() = Some(v);
        }

        // pop it back off the stack
        self.runtime.pop_stack();
    }

    fn subscribe_to(&self, source: Source) {
        self.add_source(source);
    }
}
