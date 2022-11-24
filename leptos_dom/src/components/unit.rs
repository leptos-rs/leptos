use crate::{Component, IntoNode, Node};

/// The unit `()` leptos counterpart.
pub struct Unit;

impl IntoNode for Unit {
    fn into_node(self, _: leptos_reactive::Scope) -> crate::Node {
        let component = Component::new("()");

        Node::Component(component)
    }
}
