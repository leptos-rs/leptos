use crate::{EffectId, Runtime, ScopeId};
use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct Subscriber(pub(crate) (ScopeId, EffectId));

impl Subscriber {
    pub fn run(&self, runtime: &'static Runtime) {
        runtime.any_effect(self.0, |effect| effect.run(self.0))
    }
}
