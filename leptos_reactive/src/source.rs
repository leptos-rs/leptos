use crate::{Runtime, ScopeId, SignalId, Subscriber};
use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct Source(pub(crate) (ScopeId, SignalId));

impl Source {
    pub fn unsubscribe(&self, runtime: &'static Runtime, subscriber: Subscriber) {
        runtime.any_signal(self.0, |signal_state| signal_state.unsubscribe(subscriber))
    }
}
