use leptos_reactive::Scope;
use wasm_bindgen::{JsCast, JsValue, UnwrapThrowExt};

use crate::{
    append_child, create_text_node, debug_warn, insert_before, reconcile::reconcile_arrays,
    remove_attribute, remove_child, replace_child, replace_with, set_attribute, Attribute, Child,
    Class, Property,
};

pub fn attribute<'a>(
    cx: Scope<'a>,
    el: &web_sys::Element,
    attr_name: &'static str,
    value: Attribute<'a>,
) {
    match value {
        Attribute::Fn(f) => {
            let el = el.clone();
            cx.create_effect(move || attribute_expression(&el, attr_name, f()))
        }
        _ => attribute_expression(el, attr_name, value),
    }
}

fn attribute_expression<'a>(el: &web_sys::Element, attr_name: &str, value: Attribute<'a>) {
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

pub fn property<'a>(
    cx: Scope<'a>,
    el: &web_sys::Element,
    prop_name: &'static str,
    value: Property<'a>,
) {
    match value {
        Property::Fn(f) => {
            let el = el.clone();
            cx.create_effect(move || property_expression(&el, prop_name, f()))
        }
        Property::Value(value) => property_expression(el, prop_name, value),
    }
}

fn property_expression(el: &web_sys::Element, prop_name: &str, value: JsValue) {
    js_sys::Reflect::set(el, &JsValue::from_str(prop_name), &value).unwrap_throw();
}

pub fn class<'a>(cx: Scope<'a>, el: &web_sys::Element, class_name: &'static str, value: Class<'a>) {
    match value {
        Class::Fn(f) => {
            let el = el.clone();
            cx.create_effect(move || class_expression(&el, class_name, f()))
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

pub fn insert<'a>(
    cx: Scope,
    parent: web_sys::Node,
    value: Child<'a>,
    before: Option<web_sys::Node>,
    initial: Option<Child<'a>>,
) {
    /* let initial = if before.is_some() && initial.is_none() {
        Some(Child::Nodes(vec![]))
    } else {
        initial
    }; */

    /*     while let Some(Child::Fn(f)) = current {
        current = Some(f());
    } */

    match value {
        Child::Fn(f) => {
            let mut current = initial.clone();
            cx.create_effect(move || {
                let mut value = f();
                while let Child::Fn(f) = value {
                    value = f();
                }

                insert_expression(
                    parent.clone().unchecked_into(),
                    &f(),
                    current.clone().unwrap_or(Child::Null),
                    //current.get_untracked().clone(), // get untracked to avoid infinite loop when we set current, below
                    before.as_ref(),
                );

                current = Some(value);
            });
        }
        _ => {
            insert_expression(
                parent.unchecked_into(),
                &value,
                initial.clone().unwrap_or(Child::Null),
                before.as_ref(),
            );
        }
    }
}

pub fn insert_expression<'a>(
    parent: web_sys::Element,
    new_value: &Child<'a>,
    mut current: Child<'a>,
    before: Option<&web_sys::Node>,
) -> Child<'a> {
    crate::warn!(
        "insert {:?} on {} to replace {:?}",
        new_value,
        parent.node_name(),
        current
    );

    if new_value == &current {
        current
    } else {
        let multi = before.is_some();
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
                    clean_children(&parent, current, before, None)
                }
            }
            // if it's a new text value, set that text value
            Child::Text(data) => insert_str(&parent, data, before, multi, current),
            Child::Node(node) => match current {
                Child::Nodes(current) => {
                    clean_children(&parent, Child::Nodes(current), before, Some(node.clone()))
                }
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
                    crate::warn!(
                        "replacing old node with new node\n\nparents are {} and {}",
                        old_node.parent_node().unwrap().node_name(),
                        node.parent_node().unwrap().node_name()
                    );
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
                    clean_children(&parent, current, before, None)
                } else if let Child::Nodes(ref mut current_nodes) = current {
                    if current_nodes.is_empty() {
                        Child::Nodes(append_nodes(&parent, new_nodes, before))
                    } else {
                        reconcile_arrays(&parent, current_nodes, new_nodes);
                        Child::Nodes(new_nodes.to_vec())
                    }
                } else {
                    clean_children(&parent, Child::Null, None, None);
                    append_nodes(&parent, new_nodes, before);
                    Child::Nodes(new_nodes.to_vec())
                }
            }

            // Nested Signals here simply won't do anything; they should be flattened so it's a single Signal
            Child::Fn(_) => {
                debug_warn!(
                    "{}: Child<Fn<'a, Child<Fn<'a, ...>>> should be flattened.",
                    std::panic::Location::caller()
                );
                current
            }
        }
    }
}

fn node_list_to_vec(node_list: web_sys::NodeList) -> Vec<web_sys::Node> {
    let mut vec = Vec::new();
    for idx in 0..node_list.length() {
        if let Some(node) = node_list.item(idx) {
            vec.push(node);
        }
    }
    vec
}

pub fn insert_str<'a>(
    parent: &web_sys::Element,
    data: &str,
    before: Option<&web_sys::Node>,
    multi: bool,
    current: Child,
) -> Child<'a> {
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
        clean_children(parent, current, before, Some(node))
    } else {
        match current {
            Child::Text(_) => match before {
                Some(marker) => {
                    let prev = marker.previous_sibling().unwrap_throw();
                    if let Some(text_node) = prev.dyn_ref::<web_sys::Text>() {
                        crate::log!("branch A");
                        text_node.set_data(data)
                    } else {
                        crate::log!("branch B");

                        prev.set_text_content(Some(data))
                    }
                }
                None => match parent.first_child() {
                    Some(child) => {
                        crate::log!("branch C");

                        child.unchecked_ref::<web_sys::Text>().set_data(data);
                    }
                    None => {
                        crate::log!("branch D");

                        parent.set_text_content(Some(data))
                    }
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

fn clean_children<'a>(
    parent: &web_sys::Element,
    current: Child,
    marker: Option<&web_sys::Node>,
    replacement: Option<web_sys::Node>,
) -> Child<'a> {
    match marker {
        None => {
            parent.set_text_content(Some(""));
            Child::Null
        }
        Some(marker) => {
            let node = replacement.unwrap_or_else(|| create_text_node("").unchecked_into());

            match current {
                Child::Null => Child::Node(insert_before(parent, &node, Some(marker))),
                Child::Text(_) => Child::Node(insert_before(parent, &node, Some(marker))),
                Child::Node(node) => Child::Node(insert_before(parent, &node, Some(marker))),
                Child::Nodes(nodes) => {
                    let mut inserted = false;
                    let mut result = Vec::new();
                    for (idx, el) in nodes.iter().enumerate().rev() {
                        if &node != el {
                            let is_parent =
                                el.parent_node() == Some(parent.clone().unchecked_into());
                            if !inserted && idx == 0 {
                                if is_parent {
                                    replace_child(parent, &node, el);
                                    result.push(node.clone())
                                } else {
                                    result.push(insert_before(parent, &node, Some(marker)))
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
}
