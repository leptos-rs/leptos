#![forbid(unsafe_code)]
#![allow(missing_docs)]

use crate::{
    diagnostics,
    diagnostics::*,
    node::NodeId,
    runtime::{with_runtime, RuntimeId},
    Runtime, Scope, ScopeProperty, SignalGet, SignalSet, SignalUpdate,
};

pub trait TriggerDirty {
    fn dirty(&self);
    fn try_dirty(&self) -> bool;
}

pub trait TriggerTrack {
    fn track(&self);
    fn try_track(&self) -> bool;
}

#[derive(Clone, Copy)]
pub struct Trigger {
    pub(crate) runtime: RuntimeId,
    pub(crate) id: NodeId,

    #[cfg(debug_assertions)]
    pub(crate) defined_at: &'static std::panic::Location<'static>,
}

impl TriggerDirty for Trigger {
    fn dirty(&self) {
        assert!(self.try_dirty(), "Trigger::dirty(): runtime not alive")
    }

    fn try_dirty(&self) -> bool {
        with_runtime(self.runtime, |runtime| {
            runtime.mark_dirty(self.id);
            runtime.run_your_effects();
        })
        .is_ok()
    }
}

impl TriggerTrack for Trigger {
    fn track(&self) {
        assert!(self.try_track(), "Trigger::track(): runtime not alive")
    }

    fn try_track(&self) -> bool {
        let diagnostics = diagnostics!(self);

        with_runtime(self.runtime, |runtime| {
            self.id.subscribe(runtime, diagnostics);
        })
        .is_ok()
    }
}

#[cfg_attr(
    debug_assertions,
    instrument(
        level = "trace",
        skip_all,
        fields(scope = ?cx.id)
    )
)]
#[track_caller]
pub fn create_trigger(cx: Scope) -> Trigger {
    let t = cx.runtime.create_trigger();
    cx.push_scope_property(ScopeProperty::Trigger(t.id));
    t
}

impl SignalGet<()> for Trigger {
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

impl SignalUpdate<()> for Trigger {
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
        self.dirty();
        f(&mut ())
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
        if !self.try_dirty() {
            return None;
        }

        Some(f(&mut ()))
    }
}

impl SignalSet<()> for Trigger {
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
        self.dirty();
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
        self.try_dirty().then_some(())
    }
}
