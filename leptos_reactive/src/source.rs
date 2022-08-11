use crate::{MemoId, Runtime, ScopeId, SignalId, Subscriber};
use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) enum Source {
    Signal((ScopeId, SignalId)),
    Memo((ScopeId, MemoId)),
}

impl Source {
    pub fn unsubscribe(&self, runtime: &'static Runtime, subscriber: Subscriber) {
        match self {
            Source::Signal(id) => {
                runtime.any_signal(*id, |signal_state| signal_state.unsubscribe(subscriber))
            }
            Source::Memo(id) => {
                runtime.any_memo(*id, |memo_state| memo_state.unsubscribe(subscriber))
            }
        }
    }
}
