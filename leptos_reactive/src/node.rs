use crate::{
    arena::NodeId, runtime::ThreadArena, with_runtime, AnyComputation,
};
use std::rc::Rc;

/// Handle to dispose of a reactive node.
#[derive(Debug, PartialEq, Eq)]
pub struct Disposer(pub(crate) NodeId);

impl Drop for Disposer {
    fn drop(&mut self) {
        let id = self.0;
        _ = with_runtime(|runtime| {
            ThreadArena::remove(&id);
            runtime.cleanup_node(id);
            runtime.dispose_node(id);
        });
    }
}

#[derive(Clone)]
pub(crate) struct ReactiveNode {
    pub state: ReactiveNodeState,
    pub node_type: ReactiveNodeType,
}

#[derive(Clone)]
pub(crate) enum ReactiveNodeType {
    Trigger,
    Signal,
    Memo { f: Rc<dyn AnyComputation> },
    Effect { f: Rc<dyn AnyComputation> },
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) enum ReactiveNodeState {
    Clean,
    Check,
    Dirty,

    /// Dirty and Marked as visited
    DirtyMarked,
}
