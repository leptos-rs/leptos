use crate::{EffectId, ReactiveSystemErr, Runtime, ScopeId};
use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct Subscriber(pub(crate) (ScopeId, EffectId));

impl Subscriber {
    pub fn try_run(
        &self,
        runtime: &'static Runtime,
    ) -> Result<Result<(), ReactiveSystemErr>, ReactiveSystemErr> {
        crate::debug_warn!("(Subscriber::run) {:?}", self.0);
        runtime.try_any_effect(self.0, |effect| effect.run(self.0))
    }
}
