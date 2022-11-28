#[cfg(all(target_arch = "wasm32", feature = "web"))]
use crate::{mount_child, MountKind};
use crate::{ComponentRepr, IntoNode, Node};

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
  #[cfg_attr(debug_assertions, instrument(level = "trace", name = "</>", skip_all, fields(children = self.0.len())))]
  fn into_node(self, _cx: leptos_reactive::Scope) -> Node {
    let mut frag = ComponentRepr::new("");

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
