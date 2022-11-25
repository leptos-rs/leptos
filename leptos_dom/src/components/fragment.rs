use crate::{Component, IntoNode, Node};

/// Represents a group of [`Nodes`](Node).
pub struct Fragment(Vec<Node>);

impl Fragment {
    /// Creates a new [`Fragment`] from a [`Vec<Node>`].
    pub fn new(nodes: Vec<Node>) -> Self {
        Self(nodes)
    }
}

impl IntoNode for Fragment {
    fn into_node(self, cx: leptos_reactive::Scope) -> Node {
        let frag = Component::new("Fragment");

        *frag.children.borrow_mut() = self.0;

        Node::Component(frag)
    }
}
