use crate::View;
pub(crate) use leptos_debugger::{
    update_view, ComponentMessage, DynChildMessage, RootMessage,
};
use leptos_debugger::{EachMessage, ElementMessage, TextMessage, UnitMessage};

pub(crate) fn insert_view(view: &View, parent_id: String) {
    match view {
        View::Element(el) => {
            leptos_debugger::update_view(
                ElementMessage::Create {
                    parent_id,
                    id: format!("{}", el.id),
                }
                .into(),
            );
        }
        View::Text(text) => {
            leptos_debugger::update_view(
                TextMessage::Create {
                    parent_id,
                    content: text.content.to_string(),
                }
                .into(),
            );
        }
        View::Component(comp) => {
            leptos_debugger::update_view(
                ComponentMessage::Create {
                    parent_id,
                    id: format!("{}", comp.id),
                    name: comp.name().to_string(),
                }
                .into(),
            );
        }
        View::CoreComponent(comp) => match comp {
            crate::CoreComponent::Unit(_) => {
                leptos_debugger::update_view(
                    UnitMessage::Create { parent_id }.into(),
                );
            }
            crate::CoreComponent::DynChild(child) => {
                leptos_debugger::update_view(
                    DynChildMessage::Create {
                        parent_id,
                        id: format!("{}", child.id),
                    }
                    .into(),
                );
            }
            crate::CoreComponent::Each(each) => leptos_debugger::update_view(
                EachMessage::Create {
                    parent_id,
                    id: each.id.to_string(),
                }
                .into(),
            ),
        },
        View::Transparent(_) => {}
        View::Suspense(_, _) => {}
    }
}
