use crate::AnyComputation;
use std::{any::Any, cell::RefCell, rc::Rc};

slotmap::new_key_type! {
    /// Unique ID assigned to a signal.
    pub struct NodeId;
}

#[derive(Clone)]
pub(crate) struct ReactiveNode {
    pub value: Rc<RefCell<dyn Any>>,
    pub state: ReactiveNodeState,
    pub node_type: ReactiveNodeType,
}

#[derive(Clone)]
pub(crate) enum ReactiveNodeType {
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
