use crate::{node::NodeId, with_runtime, Disposer, Runtime, SignalDispose};
use cfg_if::cfg_if;
use std::{any::Any, cell::RefCell, marker::PhantomData, rc::Rc};

/// Effects run a certain chunk of code whenever the signals they depend on change.
/// `create_effect` queues the given function to run once, tracks its dependence
/// on any signal values read within it, and reruns the function whenever the value
/// of a dependency changes.
///
/// Effects are intended to run *side-effects* of the system, not to synchronize state
/// *within* the system. In other words: don't write to signals within effects, unless
/// you’re coordinating with some other non-reactive side effect.
/// (If you need to define a signal that depends on the value of other signals, use a
/// derived signal or [`create_memo`](crate::create_memo)).
///
/// This first run is queued for the next microtask, i.e., it runs after all other
/// synchronous code has completed. In practical terms, this means that if you use
/// `create_effect` in the body of the component, it will run *after* the view has been
/// created and (presumably) mounted. (If you need an effect that runs immediately, use
/// [`create_render_effect`].)
///
/// The effect function is called with an argument containing whatever value it returned
/// the last time it ran. On the initial run, this is `None`.
///
/// By default, effects **do not run on the server**. This means you can call browser-specific
/// APIs within the effect function without causing issues. If you need an effect to run on
/// the server, use [`create_isomorphic_effect`].
/// ```
/// # use leptos_reactive::*;
/// # use log::*;
/// # let runtime = create_runtime();
/// let (a, set_a) = create_signal(0);
/// let (b, set_b) = create_signal(0);
///
/// // ✅ use effects to interact between reactive state and the outside world
/// create_effect(move |_| {
///   // immediately prints "Value: 0" and subscribes to `a`
///   log::debug!("Value: {}", a.get());
/// });
///
/// set_a.set(1);
/// // ✅ because it's subscribed to `a`, the effect reruns and prints "Value: 1"
///
/// // ❌ don't use effects to synchronize state within the reactive system
/// create_effect(move |_| {
///   // this technically works but can cause unnecessary re-renders
///   // and easily lead to problems like infinite loops
///   set_b.set(a.get() + 1);
/// });
/// # if !cfg!(feature = "ssr") {
/// # assert_eq!(b.get(), 2);
/// # }
/// # runtime.dispose();
/// ```
#[cfg_attr(
    any(debug_assertions, feature="ssr"),
    instrument(
        level = "trace",
        skip_all,
        fields(
            ty = %std::any::type_name::<T>()
        )
    )
)]
#[track_caller]
#[inline(always)]
pub fn create_effect<T>(f: impl Fn(Option<T>) -> T + 'static) -> Effect<T>
where
    T: 'static,
{
    cfg_if! {
        if #[cfg(not(feature = "ssr"))] {
            use crate::{Owner, queue_microtask, with_owner};

            let runtime = Runtime::current();
            let owner = Owner::current();
            let id = runtime.create_effect(f);

            queue_microtask(move || {
                with_owner(owner.unwrap(), move || {
                    _ = with_runtime( |runtime| {
                        runtime.update_if_necessary(id);
                    });
                });
            });

            Effect { id, ty: PhantomData }
        } else {
            // clear warnings
            _ = f;
            Effect { id: Default::default(), ty: PhantomData }
        }
    }
}

impl<T> Effect<T>
where
    T: 'static,
{
    /// Effects run a certain chunk of code whenever the signals they depend on change.
    /// `create_effect` immediately runs the given function once, tracks its dependence
    /// on any signal values read within it, and reruns the function whenever the value
    /// of a dependency changes.
    ///
    /// Effects are intended to run *side-effects* of the system, not to synchronize state
    /// *within* the system. In other words: don't write to signals within effects.
    /// (If you need to define a signal that depends on the value of other signals, use a
    /// derived signal or [`create_memo`](crate::create_memo)).
    ///
    /// The effect function is called with an argument containing whatever value it returned
    /// the last time it ran. On the initial run, this is `None`.
    ///
    /// By default, effects **do not run on the server**. This means you can call browser-specific
    /// APIs within the effect function without causing issues. If you need an effect to run on
    /// the server, use [`create_isomorphic_effect`].
    /// ```
    /// # use leptos_reactive::*;
    /// # use log::*;
    /// # let runtime = create_runtime();
    /// let a = RwSignal::new(0);
    /// let b = RwSignal::new(0);
    ///
    /// // ✅ use effects to interact between reactive state and the outside world
    /// Effect::new(move |_| {
    ///   // immediately prints "Value: 0" and subscribes to `a`
    ///   log::debug!("Value: {}", a.get());
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
    /// # if !cfg!(feature = "ssr") {
    /// # assert_eq!(b.get(), 2);
    /// # }
    /// # runtime.dispose();
    /// ```
    #[track_caller]
    #[inline(always)]
    pub fn new(f: impl Fn(Option<T>) -> T + 'static) -> Self {
        create_effect(f)
    }

    /// Creates an effect; unlike effects created by [`create_effect`], isomorphic effects will run on
    /// the server as well as the client.
    /// ```
    /// # use leptos_reactive::*;
    /// # use log::*;
    /// # let runtime = create_runtime();
    /// let a = RwSignal::new(0);
    /// let b = RwSignal::new(0);
    ///
    /// // ✅ use effects to interact between reactive state and the outside world
    /// Effect::new_isomorphic(move |_| {
    ///   // immediately prints "Value: 0" and subscribes to `a`
    ///   log::debug!("Value: {}", a.get());
    /// });
    ///
    /// a.set(1);
    /// // ✅ because it's subscribed to `a`, the effect reruns and prints "Value: 1"
    ///
    /// // ❌ don't use effects to synchronize state within the reactive system
    /// Effect::new_isomorphic(move |_| {
    ///   // this technically works but can cause unnecessary re-renders
    ///   // and easily lead to problems like infinite loops
    ///   b.set(a.get() + 1);
    /// });
    /// # assert_eq!(b.get(), 2);
    /// # runtime.dispose();
    #[track_caller]
    #[inline(always)]
    pub fn new_isomorphic(f: impl Fn(Option<T>) -> T + 'static) -> Self {
        create_isomorphic_effect(f)
    }

    /// Applies the given closure to the most recent value of the effect.
    ///
    /// Because effect functions can return values, each time an effect runs it
    /// consumes its previous value. This allows an effect to store additional state
    /// (like a DOM node, a timeout handle, or a type that implements `Drop`) and
    /// keep it alive across multiple runs.
    ///
    /// This method allows access to the effect’s value outside the effect function.
    /// The next time a signal change causes the effect to run, it will receive the
    /// mutated value.
    pub fn with_value_mut<U>(
        &self,
        f: impl FnOnce(&mut Option<T>) -> U,
    ) -> Option<U> {
        with_runtime(|runtime| {
            let nodes = runtime.nodes.borrow();
            let node = nodes.get(self.id)?;
            let value = node.value.clone()?;
            let mut value = value.borrow_mut();
            let value = value.downcast_mut()?;
            Some(f(value))
        })
        .ok()
        .flatten()
    }
}

/// Creates an effect; unlike effects created by [`create_effect`], isomorphic effects will run on
/// the server as well as the client.
/// ```
/// # use leptos_reactive::*;
/// # use log::*;
/// # let runtime = create_runtime();
/// let (a, set_a) = create_signal(0);
/// let (b, set_b) = create_signal(0);
///
/// // ✅ use effects to interact between reactive state and the outside world
/// create_isomorphic_effect(move |_| {
///   // immediately prints "Value: 0" and subscribes to `a`
///   log::debug!("Value: {}", a.get());
/// });
///
/// set_a.set(1);
/// // ✅ because it's subscribed to `a`, the effect reruns and prints "Value: 1"
///
/// // ❌ don't use effects to synchronize state within the reactive system
/// create_isomorphic_effect(move |_| {
///   // this technically works but can cause unnecessary re-renders
///   // and easily lead to problems like infinite loops
///   set_b.set(a.get() + 1);
/// });
/// # assert_eq!(b.get(), 2);
/// # runtime.dispose();
#[cfg_attr(
    any(debug_assertions, feature="ssr"),
    instrument(
        level = "trace",
        skip_all,
        fields(
            ty = %std::any::type_name::<T>()
        )
    )
)]
#[track_caller]
#[inline(always)]
pub fn create_isomorphic_effect<T>(
    f: impl Fn(Option<T>) -> T + 'static,
) -> Effect<T>
where
    T: 'static,
{
    let runtime = Runtime::current();
    let id = runtime.create_effect(f);
    //crate::macros::debug_warn!("creating effect {e:?}");
    _ = with_runtime(|runtime| {
        runtime.update_if_necessary(id);
    });
    Effect {
        id,
        ty: PhantomData,
    }
}

/// Creates an effect exactly like [`create_effect`], but runs immediately rather
/// than being queued until the end of the current microtask. This is mostly used
/// inside the renderer but is available for use cases in which scheduling the effect
/// for the next tick is not optimal.
#[cfg_attr(
    any(debug_assertions, feature="ssr"),
    instrument(
        level = "trace",
        skip_all,
        fields(
            ty = %std::any::type_name::<T>()
        )
    )
)]
#[inline(always)]
pub fn create_render_effect<T>(
    f: impl Fn(Option<T>) -> T + 'static,
) -> Effect<T>
where
    T: 'static,
{
    cfg_if! {
        if #[cfg(not(feature = "ssr"))] {
            let runtime = Runtime::current();
            let id = runtime.create_effect(f);
            _ = with_runtime( |runtime| {
                runtime.update_if_necessary(id);
            });
            Effect { id, ty: PhantomData }
        } else {
            // clear warnings
            _ = f;
            Effect { id: Default::default(), ty: PhantomData }
        }
    }
}

/// A handle to an effect, can be used to explicitly dispose of the effect.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Effect<T> {
    pub(crate) id: NodeId,
    ty: PhantomData<T>,
}

impl<T> From<Effect<T>> for Disposer {
    fn from(effect: Effect<T>) -> Self {
        Disposer(effect.id)
    }
}

impl<T> SignalDispose for Effect<T> {
    fn dispose(self) {
        drop(Disposer::from(self));
    }
}

pub(crate) struct EffectState<T, F>
where
    T: 'static,
    F: Fn(Option<T>) -> T,
{
    pub(crate) f: F,
    pub(crate) ty: PhantomData<T>,
    #[cfg(any(debug_assertions, feature = "ssr"))]
    pub(crate) defined_at: &'static std::panic::Location<'static>,
}

pub(crate) trait AnyComputation {
    fn run(&self, value: Rc<RefCell<dyn Any>>) -> bool;
}

impl<T, F> AnyComputation for EffectState<T, F>
where
    T: 'static,
    F: Fn(Option<T>) -> T,
{
    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(
            name = "Effect::run()",
            level = "trace",
            skip_all,
            fields(
              defined_at = %self.defined_at,
              ty = %std::any::type_name::<T>()
            )
        )
    )]
    fn run(&self, value: Rc<RefCell<dyn Any>>) -> bool {
        // we defensively take and release the BorrowMut twice here
        // in case a change during the effect running schedules a rerun
        // ideally this should never happen, but this guards against panic
        let curr_value = {
            // downcast value
            let mut value = value.borrow_mut();
            let value = value
                .downcast_mut::<Option<T>>()
                .expect("to downcast effect value");
            value.take()
        };

        // run the effect
        let new_value = (self.f)(curr_value);

        // set new value
        let mut value = value.borrow_mut();
        let value = value
            .downcast_mut::<Option<T>>()
            .expect("to downcast effect value");
        *value = Some(new_value);

        true
    }
}
