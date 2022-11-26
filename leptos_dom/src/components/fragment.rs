use crate::{mount_child, Component, IntoNode, MountKind, Node};

/// Represents a group of [`Nodes`](Node).
#[derive(Debug)]
pub struct Fragment(Vec<Node>);

impl Fragment {
    /// Creates a new [`Fragment`] from a [`Vec<Node>`].
    pub fn new(nodes: Vec<Node>) -> Self {
        Self(nodes)
    }
}

impl IntoNode for Fragment {
    #[instrument(level = "trace")]
    fn into_node(self, _cx: leptos_reactive::Scope) -> Node {
        let mut frag = Component::new("");

        #[cfg(all(target_arch = "wasm32", feature = "web"))]
        let closing = &frag.closing.node;

        #[cfg(all(target_arch = "wasm32", feature = "web"))]
        for child in &self.0 {
            mount_child(MountKind::Component(closing), child);
        }

        frag.children = self.0;

        Node::Component(frag)
    }
}
