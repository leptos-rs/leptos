use crate::{with_runtime, AnyComputation};
use std::{any::Any, cell::RefCell, rc::Rc};

slotmap::new_key_type! {
    /// Unique ID assigned to a signal.
    pub struct NodeId;
}

/// Handle to dispose of a reactive node.
#[derive(Debug, PartialEq, Eq)]
pub struct Disposer(pub(crate) NodeId);

impl Drop for Disposer {
    fn drop(&mut self) {
        let id = self.0;
        _ = with_runtime(|runtime| {
            runtime.cleanup_node(id);
            runtime.dispose_node(id);
        });
    }
}

#[derive(Clone)]
pub(crate) struct ReactiveNode {
    pub value: Option<Rc<RefCell<dyn Any>>>,
    pub state: ReactiveNodeState,
    pub node_type: ReactiveNodeType,
}

impl ReactiveNode {
    pub fn value(&self) -> Rc<RefCell<dyn Any>> {
        self.value
            .clone()
            .expect("ReactiveNode.value to have a value")
    }
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
