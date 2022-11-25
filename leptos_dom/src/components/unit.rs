use crate::{Component, IntoNode, Node};

/// The unit `()` leptos counterpart.
#[derive(Clone, Copy, Debug)]
pub struct Unit;

impl IntoNode for Unit {
    #[instrument(level = "trace")]
    fn into_node(self, _: leptos_reactive::Scope) -> crate::Node {
        let component = Component::new("()");

        Node::Component(component)
    }
}
