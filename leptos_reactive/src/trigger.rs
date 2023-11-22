use crate::{
    diagnostics,
    diagnostics::*,
    node::NodeId,
    runtime::{with_runtime, Runtime},
    SignalGet, SignalSet, SignalUpdate,
};

/// Reactive Trigger, notifies reactive code to rerun.
///
/// See [`create_trigger`] for more.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Trigger {
    pub(crate) id: NodeId,

    #[cfg(debug_assertions)]
    pub(crate) defined_at: &'static std::panic::Location<'static>,
}

impl Trigger {
    /// Creates a [`Trigger`](crate::Trigger), a kind of reactive primitive.
    ///
    /// A trigger is a data-less signal with the sole purpose
    /// of notifying other reactive code of a change. This can be useful
    /// for when using external data not stored in signals, for example.
    ///
    /// This is identical to [`create_trigger`].
    ///
    /// ```
    /// # use leptos_reactive::*;
    /// # let runtime = create_runtime();
    /// use std::{cell::RefCell, fmt::Write, rc::Rc};
    ///
    /// let external_data = Rc::new(RefCell::new(1));
    /// let output = Rc::new(RefCell::new(String::new()));
    ///
    /// let rerun_on_data = Trigger::new();
    ///
    /// let o = output.clone();
    /// let e = external_data.clone();
    /// create_effect(move |_| {
    ///     // can be `rerun_on_data()` on nightly
    ///     rerun_on_data.track();
    ///     write!(o.borrow_mut(), "{}", *e.borrow());
    ///     *e.borrow_mut() += 1;
    /// });
    /// # if !cfg!(feature = "ssr") {
    /// assert_eq!(*output.borrow(), "1");
    ///
    /// rerun_on_data.notify(); // reruns the above effect
    ///
    /// assert_eq!(*output.borrow(), "12");
    /// # }
    /// # runtime.dispose();
    /// ```
    #[inline(always)]
    #[track_caller]
    pub fn new() -> Self {
        create_trigger()
    }

    /// Notifies any reactive code where this trigger is tracked to rerun.
    ///
    /// ## Panics
    /// Panics if there is no current reactive runtime, or if the
    /// trigger has been disposed.
    pub fn notify(&self) {
        assert!(self.try_notify(), "Trigger::notify(): runtime not alive")
    }

    /// Attempts to notify any reactive code where this trigger is tracked to rerun.
    ///
    /// Returns `false` if there is no current reactive runtime.
    pub fn try_notify(&self) -> bool {
        with_runtime(|runtime| {
            runtime.mark_dirty(self.id);
            runtime.run_effects();
        })
        .is_ok()
    }

    /// Subscribes the running effect to this trigger.
    ///
    /// ## Panics
    /// Panics if there is no current reactive runtime, or if the
    /// trigger has been disposed.
    pub fn track(&self) {
        assert!(self.try_track(), "Trigger::track(): runtime not alive")
    }

    /// Attempts to subscribe the running effect to this trigger, returning
    /// `false` if there is no current reactive runtime.
    pub fn try_track(&self) -> bool {
        let diagnostics = diagnostics!(self);

        with_runtime(|runtime| {
            runtime.update_if_necessary(self.id);
            self.id.subscribe(runtime, diagnostics);
        })
        .is_ok()
    }
}

/// Creates a [`Trigger`](crate::Trigger), a kind of reactive primitive.
///
/// A trigger is a data-less signal with the sole purpose
/// of notifying other reactive code of a change. This can be useful
/// for when using external data not stored in signals, for example.
/// ```
/// # use leptos_reactive::*;
/// # let runtime = create_runtime();
/// use std::{cell::RefCell, fmt::Write, rc::Rc};
///
/// let external_data = Rc::new(RefCell::new(1));
/// let output = Rc::new(RefCell::new(String::new()));
///
/// let rerun_on_data = create_trigger();
///
/// let o = output.clone();
/// let e = external_data.clone();
/// create_effect(move |_| {
///     // can be `rerun_on_data()` on nightly
///     rerun_on_data.track();
///     write!(o.borrow_mut(), "{}", *e.borrow());
///     *e.borrow_mut() += 1;
/// });
/// # if !cfg!(feature = "ssr") {
/// assert_eq!(*output.borrow(), "1");
///
/// rerun_on_data.notify(); // reruns the above effect
///
/// assert_eq!(*output.borrow(), "12");
/// # }
/// # runtime.dispose();
/// ```
#[cfg_attr(debug_assertions, instrument(level = "trace", skip_all,))]
#[track_caller]
pub fn create_trigger() -> Trigger {
    Runtime::current().create_trigger()
}

impl Default for Trigger {
    fn default() -> Self {
        Self::new()
    }
}

impl SignalGet for Trigger {
    type Value = ();

    #[cfg_attr(
        debug_assertions,
        instrument(
            level = "trace",
            name = "Trigger::get()",
            skip_all,
            fields(
                id = ?self.id,
                defined_at = %self.defined_at
            )
        )
    )]
    #[track_caller]
    #[inline(always)]
    fn get(&self) {
        self.track()
    }

    #[cfg_attr(
        debug_assertions,
        instrument(
            level = "trace",
            name = "Trigger::try_get()",
            skip_all,
            fields(
                id = ?self.id,
                defined_at = %self.defined_at
            )
        )
    )]
    #[inline(always)]
    fn try_get(&self) -> Option<()> {
        self.try_track().then_some(())
    }
}

impl SignalUpdate for Trigger {
    type Value = ();

    #[cfg_attr(
        debug_assertions,
        instrument(
            name = "Trigger::update()",
            level = "trace",
            skip_all,
            fields(
                id = ?self.id,
                defined_at = %self.defined_at
            )
        )
    )]
    #[inline(always)]
    fn update(&self, f: impl FnOnce(&mut ())) {
        self.try_update(f).expect("runtime to be alive")
    }

    #[cfg_attr(
        debug_assertions,
        instrument(
            name = "Trigger::try_update()",
            level = "trace",
            skip_all,
            fields(
                id = ?self.id,
                defined_at = %self.defined_at
            )
        )
    )]
    #[inline(always)]
    fn try_update<O>(&self, f: impl FnOnce(&mut ()) -> O) -> Option<O> {
        // run callback with runtime before dirtying the trigger,
        // consistent with signals.
        with_runtime(|runtime| {
            let res = f(&mut ());

            runtime.mark_dirty(self.id);
            runtime.run_effects();

            Some(res)
        })
        .ok()
        .flatten()
    }
}

impl SignalSet for Trigger {
    type Value = ();

    #[cfg_attr(
        debug_assertions,
        instrument(
            level = "trace",
            name = "Trigger::set()",
            skip_all,
            fields(
                id = ?self.id,
                defined_at = %self.defined_at
            )
        )
    )]
    #[inline(always)]
    fn set(&self, _: ()) {
        self.notify();
    }

    #[cfg_attr(
        debug_assertions,
        instrument(
            level = "trace",
            name = "Trigger::try_set()",
            skip_all,
            fields(
                id = ?self.id,
                defined_at = %self.defined_at
            )
        )
    )]
    #[inline(always)]
    fn try_set(&self, _: ()) -> Option<()> {
        self.try_notify().then_some(())
    }
}

#[cfg(feature = "nightly")]
impl FnOnce<()> for Trigger {
    type Output = ();

    #[inline(always)]
    extern "rust-call" fn call_once(self, _args: ()) -> Self::Output {
        self.track()
    }
}

#[cfg(feature = "nightly")]
impl FnMut<()> for Trigger {
    #[inline(always)]
    extern "rust-call" fn call_mut(&mut self, _args: ()) -> Self::Output {
        self.track()
    }
}

#[cfg(feature = "nightly")]
impl Fn<()> for Trigger {
    #[inline(always)]
    extern "rust-call" fn call(&self, _args: ()) -> Self::Output {
        self.track()
    }
}
