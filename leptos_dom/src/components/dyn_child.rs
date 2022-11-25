use std::borrow::Cow;

use leptos_reactive::{create_effect, Scope};

use crate::{mount_child, Component, IntoNode, MountKind, Node};

/// Represents any [`Node`] that can change over time.
pub struct DynChild<CF, N>
where
    CF: Fn() -> N + 'static,
    N: IntoNode,
{
    name: Cow<'static, str>,
    child_fn: CF,
}

impl<CF, N> DynChild<CF, N>
where
    CF: Fn() -> N + 'static,
    N: IntoNode,
{
    /// Creates a new dynamic child which will re-render whenever it's
    /// signal dependencies change.
    pub fn new(child_fn: CF) -> Self {
        Self {
            child_fn,
            name: "DynChild".into(),
        }
    }

    /// Renames this component so you can use it as a primitive for
    /// something else, such as [`DynText`](crate::DynText).
    pub fn rename(&mut self, new_name: impl Into<Cow<'static, str>>) {
        self.name = new_name.into()
    }
}

impl<CF, N> IntoNode for DynChild<CF, N>
where
    CF: Fn() -> N + 'static,
    N: IntoNode,
{
    #[instrument(level = "trace", skip_all)]
    fn into_node(self, cx: Scope) -> crate::Node {
        let Self { name, child_fn } = self;

        let component = Component::new(name);

        // Optimization so we never have to re-allocate
        *component.children.borrow_mut() = vec![().into_node(cx)];

        #[cfg(all(target_arch = "wasm32", feature = "web"))]
        let closing = component.closing.node.0.clone();
        let children = component.children.clone();

        let span = tracing::Span::current();

        create_effect(cx, move |_| {
            let _guard = span.enter();
            let _guard = trace_span!("DynChild reactive").entered();

            let new_child = child_fn().into_node(cx);

            mount_child(MountKind::Component(&closing), &new_child);

            children.borrow_mut()[0] = new_child;
        });

        Node::Component(component)
    }
}
