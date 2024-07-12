/// A node in the reactive graph.
pub trait ReactiveNode {
    /// Notifies the source's dependencies that it has changed.
    fn mark_dirty(&self);

    /// Notifies the source's dependencies that it may have changed.
    fn mark_check(&self);

    /// Marks that all subscribers need to be checked.
    fn mark_subscribers_check(&self);

    /// Regenerates the value for this node, if needed, and returns whether
    /// it has actually changed or not.
    fn update_if_necessary(&self) -> bool;
}

/// The current state of a reactive node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ReactiveNodeState {
    /// The node is known to be clean: i.e., either none of its sources have changed, or its
    /// sources have changed but its value is unchanged and its dependencies do not need to change.
    Clean,
    /// The node may have changed, but it is not yet known whether it has actually changed.
    Check,
    /// The node's value has definitely changed, and subscribers will need to update.
    Dirty,
}
