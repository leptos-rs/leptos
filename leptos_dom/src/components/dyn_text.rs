use leptos_reactive::Scope;

use crate::{IntoNode, Node, Text};

use super::DynChild;

/// Represents text which can change over time.
pub struct DynText(Node);

impl DynText {
    /// Creates a new [`DynText`] component.
    pub fn new<TF, Txt>(cx: Scope, text_fn: TF) -> Self
    where
        TF: Fn(Scope) -> Txt + 'static,
        Txt: ToString,
    {
        let mut dyn_child = DynChild::new(cx, move |cx| {
            let text = text_fn(cx).to_string();

            let text = Text::new(&text);

            Node::Text(text)
        });

        dyn_child.rename("DynText");

        Self(dyn_child.into_node(cx))
    }
}

impl IntoNode for DynText {
    fn into_node(self, _: Scope) -> Node {
        self.0
    }
}
