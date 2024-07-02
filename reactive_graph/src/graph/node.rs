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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ReactiveNodeState {
    Clean,
    Check,
    Dirty,
}
