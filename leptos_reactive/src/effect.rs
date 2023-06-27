#![forbid(unsafe_code)]
use crate::{Scope, ScopeProperty};
use cfg_if::cfg_if;
use std::{any::Any, cell::RefCell, marker::PhantomData, rc::Rc};

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
/// # create_scope(create_runtime(), |cx| {
/// let (a, set_a) = create_signal(cx, 0);
/// let (b, set_b) = create_signal(cx, 0);
///
/// // ✅ use effects to interact between reactive state and the outside world
/// create_effect(cx, move |_| {
///   // immediately prints "Value: 0" and subscribes to `a`
///   log::debug!("Value: {}", a.get());
/// });
///
/// set_a.set(1);
/// // ✅ because it's subscribed to `a`, the effect reruns and prints "Value: 1"
///
/// // ❌ don't use effects to synchronize state within the reactive system
/// create_effect(cx, move |_| {
///   // this technically works but can cause unnecessary re-renders
///   // and easily lead to problems like infinite loops
///   set_b.set(a.get() + 1);
/// });
/// # if !cfg!(feature = "ssr") {
/// # assert_eq!(b.get(), 2);
/// # }
/// # }).dispose();
/// ```
#[cfg_attr(
    any(debug_assertions, feature="ssr"),
    instrument(
        level = "trace",
        skip_all,
        fields(
            scope = ?cx.id,
            ty = %std::any::type_name::<T>()
        )
    )
)]
#[track_caller]
#[inline(always)]
pub fn create_effect<T>(cx: Scope, f: impl Fn(Option<T>) -> T + 'static)
where
    T: 'static,
{
    cfg_if! {
        if #[cfg(not(feature = "ssr"))] {
            let e = cx.runtime.create_effect(f);
            //eprintln!("created effect {e:?}");
            cx.push_scope_property(ScopeProperty::Effect(e))
        } else {
            // clear warnings
            _ = cx;
            _ = f;
        }
    }
}

/// Creates an effect; unlike effects created by [`create_effect`], isomorphic effects will run on
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
///   log::debug!("Value: {}", a.get());
/// });
///
/// set_a.set(1);
/// // ✅ because it's subscribed to `a`, the effect reruns and prints "Value: 1"
///
/// // ❌ don't use effects to synchronize state within the reactive system
/// create_isomorphic_effect(cx, move |_| {
///   // this technically works but can cause unnecessary re-renders
///   // and easily lead to problems like infinite loops
///   set_b.set(a.get() + 1);
/// });
/// # assert_eq!(b.get(), 2);
/// # }).dispose();
#[cfg_attr(
    any(debug_assertions, feature="ssr"),
    instrument(
        level = "trace",
        skip_all,
        fields(
            scope = ?cx.id,
            ty = %std::any::type_name::<T>()
        )
    )
)]
#[track_caller]
#[inline(always)]
pub fn create_isomorphic_effect<T>(
    cx: Scope,
    f: impl Fn(Option<T>) -> T + 'static,
) where
    T: 'static,
{
    let e = cx.runtime.create_effect(f);
    //eprintln!("created effect {e:?}");
    cx.push_scope_property(ScopeProperty::Effect(e))
}

#[doc(hidden)]
#[cfg_attr(
    any(debug_assertions, feature="ssr"),
    instrument(
        level = "trace",
        skip_all,
        fields(
            scope = ?cx.id,
            ty = %std::any::type_name::<T>()
        )
    )
)]
#[inline(always)]
pub fn create_render_effect<T>(cx: Scope, f: impl Fn(Option<T>) -> T + 'static)
where
    T: 'static,
{
    create_effect(cx, f);
}

pub(crate) struct Effect<T, F>
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

impl<T, F> AnyComputation for Effect<T, F>
where
    T: 'static,
    F: Fn(Option<T>) -> T,
{
    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(
            name = "Effect::run()",
            level = "debug",
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
