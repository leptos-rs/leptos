use leptos_reactive::Scope;

use crate::{IntoNode, Node, Text};

use super::DynChild;

/// Represents text which can change over time.
pub struct DynText<TF, Txt>(TF)
where
    TF: Fn() -> Txt + 'static,
    Txt: ToString;

impl<TF, Txt> DynText<TF, Txt>
where
    TF: Fn() -> Txt + 'static,
    Txt: ToString,
{
    /// Creates a new [`DynText`] component.
    pub fn new(text_fn: TF) -> Self {
        Self(text_fn)
    }
}

impl<TF, Txt> IntoNode for DynText<TF, Txt>
where
    TF: Fn() -> Txt + 'static,
    Txt: ToString,
{
    fn into_node(self, cx: Scope) -> Node {
        let mut dyn_text = DynChild::new(move || {
            let text = self.0().to_string();

            let text = Text::new(&text);

            Node::Text(text)
        });

        dyn_text.rename("DynChild");

        dyn_text.into_node(cx)
    }
}
