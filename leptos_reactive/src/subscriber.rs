use crate::{EffectId, MemoId, Runtime, ScopeId};
use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) enum Subscriber {
    Memo((ScopeId, MemoId)),
    Effect((ScopeId, EffectId)),
}

impl Subscriber {
    pub fn run(&self, runtime: &'static Runtime) {
        match self {
            Subscriber::Memo(id) => runtime.any_memo(*id, |memo| memo.run(*id)),
            Subscriber::Effect(id) => runtime.any_effect(*id, |effect| effect.run(*id)),
        }
    }
}
