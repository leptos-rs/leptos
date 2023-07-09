#[cfg(feature = "debugger")]
use crate::View;
pub use leptos_debugger::{remove_view, update_props, DNode, Prop, PropValue};

#[cfg(feature = "debugger")]
pub fn insert_view(view: &View, id: String) {
    match view {
        View::Element(el) => {
            leptos_debugger::insert_view(
                id,
                DNode::Element {
                    name: el.name.to_string(),
                    id: format!("{}", el.id),
                    children: vec![],
                },
            );
        }
        View::Text(text) => {
            leptos_debugger::insert_view(
                id,
                leptos_debugger::DNode::Text(text.content.to_string()),
            );
        }
        View::Component(comp) => {
            leptos_debugger::insert_view(
                id,
                DNode::Component {
                    id: format!("{}", comp.id),
                    name: comp.name().to_string(),
                    props: vec![],
                    children: vec![],
                },
            );
        }
        View::CoreComponent(comp) => match comp {
            crate::CoreComponent::Unit(_) => {
                leptos_debugger::insert_view(id, DNode::Unit);
            }
            crate::CoreComponent::DynChild(child) => {
                leptos_debugger::insert_view(
                    id,
                    DNode::DynChild {
                        id: format!("{}", child.id),
                        children: vec![],
                    },
                );
            }
            crate::CoreComponent::Each(_) => {}
        },
        View::Transparent(_) => {}
        View::Suspense(_, _) => {}
    }
}
