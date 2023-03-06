use std::{rc::Rc, cell::RefCell, any::Any};

use crate::{AnyEffect, AnyMemo};


slotmap::new_key_type! {
    /// Unique ID assigned to a signal.
    pub struct NodeId;
}

#[derive(Clone)]
pub(crate) struct ReactiveNode {
    pub value: Rc<RefCell<dyn Any>>,
    pub node_type: ReactiveNodeType
}

#[derive(Clone)]
pub(crate) enum ReactiveNodeType {
    Signal,
    Memo {
        state: ReactiveNodeState,
        f: Rc<dyn AnyMemo>
    },
    Effect(Rc<dyn AnyEffect>)
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) enum ReactiveNodeState {
    Clean,
    Check,
    Dirty
}
