use leptos_reactive::{create_render_effect, Scope};
use wasm_bindgen::{JsCast, JsValue, UnwrapThrowExt};

use crate::{
    append_child, create_text_node, debug_warn, insert_before, reconcile::reconcile_arrays,
    remove_attribute, remove_child, replace_child, replace_with, set_attribute, Attribute, Child,
    Class, Property,
};

pub fn attribute(cx: Scope, el: &web_sys::Element, attr_name: &'static str, value: Attribute) {
    match value {
        Attribute::Fn(f) => {
            let el = el.clone();
            create_render_effect(cx, move |_| attribute_expression(&el, attr_name, f()));
        }
        _ => attribute_expression(el, attr_name, value),
    }
}

fn attribute_expression(el: &web_sys::Element, attr_name: &str, value: Attribute) {
    match value {
        Attribute::String(value) => set_attribute(el, attr_name, &value),
        Attribute::Option(value) => match value {
            Some(value) => set_attribute(el, attr_name, &value),
            None => remove_attribute(el, attr_name),
        },
        Attribute::Bool(_) => todo!(),
        _ => panic!("Remove nested Fn in Attribute"),
    }
}

pub fn property(cx: Scope, el: &web_sys::Element, prop_name: &'static str, value: Property) {
    match value {
        Property::Fn(f) => {
            let el = el.clone();
            create_render_effect(cx, move |_| property_expression(&el, prop_name, f()));
        }
        Property::Value(value) => property_expression(el, prop_name, value),
    }
}

fn property_expression(el: &web_sys::Element, prop_name: &str, value: JsValue) {
    js_sys::Reflect::set(el, &JsValue::from_str(prop_name), &value).unwrap_throw();
}

pub fn class(cx: Scope, el: &web_sys::Element, class_name: &'static str, value: Class) {
    match value {
        Class::Fn(f) => {
            let el = el.clone();
            create_render_effect(cx, move |_| class_expression(&el, class_name, f()));
        }
        Class::Value(value) => class_expression(el, class_name, value),
    }
}

fn class_expression(el: &web_sys::Element, class_name: &str, value: bool) {
    let class_list = el.class_list();
    if value {
        class_list.add_1(class_name).unwrap_throw();
    } else {
        class_list.remove_1(class_name).unwrap_throw();
    }
}

pub fn insert(
    cx: Scope,
    parent: web_sys::Node,
    mut value: Child,
    before: Option<web_sys::Node>,
    initial: Option<Child>,
    multi: bool,
) {
    /* let initial = if before.is_some() && initial.is_none() {
        Some(Child::Nodes(vec![]))
    } else {
        initial
    }; */

    log::debug!(
        "inserting {value:?} into {} before {:?} with initial {:?}",
        parent.node_name(),
        before.as_ref().map(|n| n.node_name()),
        initial
    );

    /* while let Child::Fn(f) = value {
        value = f();
        log::debug!("insert Fn value = {value:?}");
    } */

    match value {
        Child::Fn(f) => {
            create_render_effect(cx, move |current| {
                let mut current = current
                    .unwrap_or_else(|| initial.clone())
                    .unwrap_or(Child::Null);

                let mut value = f();
                while let Child::Fn(f) = value {
                    value = f();
                }

                Some(insert_expression(
                    parent.clone().unchecked_into(),
                    &value,
                    current,
                    before.as_ref(),
                    multi,
                ))
            });
        }
        _ => {
            insert_expression(
                parent.unchecked_into(),
                &value,
                initial.unwrap_or(Child::Null),
                before.as_ref(),
                multi,
            );
        }
    }
}

pub fn insert_expression(
    parent: web_sys::Element,
    new_value: &Child,
    mut current: Child,
    before: Option<&web_sys::Node>,
    multi: bool,
) -> Child {
    log::debug!(
        "insert_expression {new_value:?} into {} before {:?} with current {current:?} and multi = {multi}",
        parent.node_name(),
        before.map(|b| b.node_name())
    );
    if new_value == &current {
        log::debug!("insert_expression: values are equal");
        current
    } else {
        let parent = if multi {
            match &current {
                Child::Nodes(nodes) => nodes
                    .get(0)
                    .and_then(|node| node.parent_node())
                    .map(|node| node.unchecked_into::<web_sys::Element>())
                    .unwrap_or_else(|| parent.clone()),
                _ => parent,
            }
        } else {
            parent
        };

        match new_value {
            // if the new value is null, clean children out of the parent up to the marker node
            Child::Null => {
                if let Child::Node(old_node) = current {
                    crate::debug_warn!("just remove the node");
                    remove_child(&parent, &old_node);
                    Child::Null
                } else {
                    clean_children(&parent, current, before, None, multi)
                }
            }
            // if it's a new text value, set that text value
            Child::Text(data) => insert_str(&parent, data, before, multi, current),
            Child::Node(node) => match current {
                Child::Nodes(current) => clean_children(
                    &parent,
                    Child::Nodes(current),
                    before,
                    Some(node.clone()),
                    multi,
                ),
                Child::Null => Child::Node(append_child(&parent, node)),
                Child::Text(current_text) => {
                    if current_text.is_empty() {
                        Child::Node(append_child(&parent, node))
                    } else {
                        replace_with(parent.first_child().unwrap_throw().unchecked_ref(), node);
                        Child::Node(node.clone())
                    }
                }
                Child::Node(old_node) => {
                    /* crate::warn!(
                        "replacing old node with new node\n\nparents are {} and {}",
                        old_node.parent_node().unwrap().node_name(),
                        node.parent_node().unwrap().node_name()
                    ); */
                    replace_with(old_node.unchecked_ref(), node);
                    Child::Node(node.clone())
                }
                Child::Fn(_) => {
                    debug_warn!(
                        "{}: replacing a Child::Node<{}> with Child::Fn<...>",
                        std::panic::Location::caller(),
                        node.node_name()
                    );
                    current
                }
            },
            Child::Nodes(new_nodes) => {
                if new_nodes.is_empty() {
                    clean_children(&parent, current, before, None, multi)
                } else if let Child::Nodes(ref mut current_nodes) = current {
                    if current_nodes.is_empty() {
                        Child::Nodes(append_nodes(&parent, new_nodes, before))
                    } else {
                        reconcile_arrays(&parent, current_nodes, new_nodes);
                        Child::Nodes(new_nodes.to_vec())
                    }
                } else {
                    clean_children(&parent, Child::Null, None, None, multi);
                    append_nodes(&parent, new_nodes, before);
                    Child::Nodes(new_nodes.to_vec())
                }
            }

            // Nested Signals here simply won't do anything; they should be flattened so it's a single Signal
            Child::Fn(f) => {
                let mut value = f();
                while let Child::Fn(f) = value {
                    value = f();
                    log::debug!("insert_expression Fn value = {value:?}");
                }
                value
            }
        }
    }
}

pub fn insert_str(
    parent: &web_sys::Element,
    data: &str,
    before: Option<&web_sys::Node>,
    multi: bool,
    current: Child,
) -> Child {
    log::debug!(
        "insert_str {data:?} into {} before {:?} with current {current:?} and multi = {multi}",
        parent.node_name(),
        before.map(|b| b.node_name())
    );

    if multi {
        let node = if let Child::Nodes(nodes) = &current {
            if let Some(node) = nodes.get(0) {
                if node.node_type() == 3 {
                    node.unchecked_ref::<web_sys::Text>().set_data(data);
                    node.clone()
                } else {
                    create_text_node(data).unchecked_into()
                }
            } else {
                create_text_node(data).unchecked_into()
            }
        } else if let Some(node) = before
            .and_then(|marker| marker.previous_sibling())
            .and_then(|prev| prev.dyn_into::<web_sys::Text>().ok())
        {
            node.set_data(data);
            return Child::Text(data.to_string());
        } else {
            create_text_node(data).unchecked_into()
        };
        clean_children(parent, current, before, Some(node), multi)
    } else {
        match current {
            Child::Text(_) => match before {
                Some(marker) => {
                    let prev = marker.previous_sibling().unwrap_throw();
                    if let Some(text_node) = prev.dyn_ref::<web_sys::Text>() {
                        text_node.set_data(data)
                    } else {
                        prev.set_text_content(Some(data))
                    }
                }
                None => match parent.first_child() {
                    Some(child) => {
                        child.unchecked_ref::<web_sys::Text>().set_data(data);
                    }
                    None => parent.set_text_content(Some(data)),
                },
            },

            /* match parent.first_child() {
                Some(child) => {
                    child.unchecked_ref::<web_sys::Text>().set_data(data);
                }
                None => parent.set_text_content(Some(data)),
            }, */
            _ => parent.set_text_content(Some(data)),
        }
        Child::Text(data.to_string())
    }
}

fn append_nodes(
    parent: &web_sys::Element,
    new_nodes: &[web_sys::Node],
    marker: Option<&web_sys::Node>,
) -> Vec<web_sys::Node> {
    let mut result = Vec::new();
    for node in new_nodes {
        if let Some(marker) = marker {
            result.push(insert_before(parent, node, Some(marker)));
        } else {
            result.push(append_child(parent, node));
        }
    }
    result
}

fn clean_children(
    parent: &web_sys::Element,
    current: Child,
    marker: Option<&web_sys::Node>,
    replacement: Option<web_sys::Node>,
    multi: bool,
) -> Child {
    log::debug!(
        "clean_children on {} before {:?} with current {current:?} and replacement {replacement:?} and multi = {multi}",
        parent.node_name(),
        marker.map(|b| b.node_name())
    );

    if marker.is_none() && !multi {
        parent.set_text_content(Some(""));
        Child::Null
    } else {
        let node = replacement.unwrap_or_else(|| create_text_node("").unchecked_into());

        match current {
            Child::Null => Child::Node(insert_before(parent, &node, marker)),
            Child::Text(_) => Child::Node(insert_before(parent, &node, marker)),
            Child::Node(node) => Child::Node(insert_before(parent, &node, marker)),
            Child::Nodes(nodes) => {
                let mut inserted = false;
                let mut result = Vec::new();
                for (idx, el) in nodes.iter().enumerate().rev() {
                    if &node != el {
                        let is_parent = el.parent_node() == Some(parent.clone().unchecked_into());
                        if !inserted && idx == 0 {
                            if is_parent {
                                replace_child(parent, &node, el);
                                result.push(node.clone())
                            } else {
                                result.push(insert_before(parent, &node, marker))
                            }
                        }
                    } else {
                        inserted = true;
                    }
                }
                Child::Nodes(result)
            }
            Child::Fn(_) => todo!(),
        }
    }
}
