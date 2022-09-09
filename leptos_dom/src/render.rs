use leptos_reactive::{create_render_effect, Scope};
use wasm_bindgen::{JsCast, JsValue, UnwrapThrowExt};

use crate::{
    append_child, create_text_node, debug_warn, insert_before, reconcile::reconcile_arrays,
    remove_attribute, remove_child, replace_child, replace_with, set_attribute, Attribute, Child,
    Class, Property,
};

#[derive(Clone, PartialEq, Eq)]
pub enum Marker {
    NoChildren,
    LastChild,
    BeforeChild(web_sys::Node),
}

impl Marker {
    fn as_some_node(&self) -> Option<&web_sys::Node> {
        match &self {
            Self::BeforeChild(node) => Some(node),
            _ => None,
        }
    }
}

impl std::fmt::Debug for Marker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoChildren => write!(f, "NoChildren"),
            Self::LastChild => write!(f, "LastChild"),
            Self::BeforeChild(arg0) => f
                .debug_tuple("BeforeChild")
                .field(&arg0.node_name())
                .finish(),
        }
    }
}

pub fn attribute(cx: Scope, el: &web_sys::Element, attr_name: &'static str, value: Attribute) {
    match value {
        Attribute::Fn(f) => {
            let el = el.clone();
            create_render_effect(cx, move |old| {
                let new = f();
                if old.as_ref() != Some(&new) {
                    attribute_expression(&el, attr_name, new.clone());
                }
                new
            });
        }
        _ => attribute_expression(el, attr_name, value),
    }
}

fn attribute_expression(el: &web_sys::Element, attr_name: &str, value: Attribute) {
    match value {
        Attribute::String(value) => {
            if attr_name == "inner_html" {
                el.set_inner_html(&value);
            } else {
                set_attribute(el, attr_name, &value)
            }
        }
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
            create_render_effect(cx, move |old| {
                let new = f();
                if old.as_ref() != Some(&new) && !(old == None && new == JsValue::UNDEFINED) {
                    property_expression(&el, prop_name, new.clone())
                }
                new
            });
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
            create_render_effect(cx, move |old| {
                let new = f();
                if old.as_ref() != Some(&new) && (old.is_some() || new) {
                    class_expression(&el, class_name, new)
                }
                new
            });
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
    value: Child,
    before: Marker,
    initial: Option<Child>,
) {
    /* log::debug!(
        "inserting {value:?} on {} before {before:?} with initial = {initial:?}",
        parent.node_name()
    ); */

    match value {
        Child::Fn(f) => {
            create_render_effect(cx, move |current| {
                let current = current
                    .unwrap_or_else(|| initial.clone())
                    .unwrap_or(Child::Null);

                let mut value = (f.borrow_mut())();
                log::debug!(
                    "rendering Child::Fn on {} — value is {:?}",
                    parent.node_name(),
                    value
                );

                if current != value {
                    while let Child::Fn(f) = value {
                        value = (f.borrow_mut())();
                    }

                    Some(insert_expression(
                        parent.clone().unchecked_into(),
                        &value,
                        current,
                        &before,
                    ))
                } else {
                    Some(current)
                }
            });
        }
        _ => {
            insert_expression(
                parent.unchecked_into(),
                &value,
                initial.unwrap_or(Child::Null),
                &before,
            );
        }
    }
}

pub fn insert_expression(
    parent: web_sys::Element,
    new_value: &Child,
    mut current: Child,
    before: &Marker,
) -> Child {
    log::debug!(
        "insert_expression {new_value:?} on {} before {before:?} with current = {current:?}\nparent.parentNode = {}",
        parent.node_name(),
        parent.parent_node().unwrap().node_name()
    );

    if new_value == &current {
        current
    } else {
        let multi = before != &Marker::NoChildren;

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
                Child::Null => {
                    log::debug!(
                        "okay, should append node {} on {} (under {:?})",
                        node.node_name(),
                        parent.node_name(),
                        parent.parent_node().map(|n| n.node_name())
                    );
                    Child::Node(append_child(&parent, node))
                }
                Child::Text(current_text) => {
                    if current_text.is_empty() {
                        Child::Node(append_child(&parent, node))
                    } else {
                        replace_with(parent.first_child().unwrap_throw().unchecked_ref(), node);
                        Child::Node(node.clone())
                    }
                }
                Child::Node(old_node) => {
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
                    log::debug!("branch C clean_children on {}", parent.node_name());
                    clean_children(&parent, Child::Null, &Marker::NoChildren, None);
                    log::debug!("branch C cleaned children on {}", parent.node_name());
                    append_nodes(&parent, new_nodes, before);
                    log::debug!(
                        "branch C append_nodes on {} {:?}",
                        parent.node_name(),
                        new_nodes
                    );
                    Child::Nodes(new_nodes.to_vec())
                }
            }

            // Nested Signals here simply won't do anything; they should be flattened so it's a single Signal
            Child::Fn(f) => {
                let mut value = (f.borrow_mut())();
                while let Child::Fn(f) = value {
                    value = (f.borrow_mut())();
                }
                value
            }
        }
    }
}

pub fn insert_str(
    parent: &web_sys::Element,
    data: &str,
    before: &Marker,
    multi: bool,
    current: Child,
) -> Child {
    /* log::debug!(
        "insert_str {data:?} on {} before {before:?} — multi = {multi} and current = {current:?}",
        parent.node_name()
    ); */

    if multi {
        if let Child::Node(node) = &current {
            if node.node_type() == 3 {
                node.unchecked_ref::<web_sys::Text>().set_data(data);
                current
            } else {
                let new_node = create_text_node(data).unchecked_into::<web_sys::Node>();
                replace_child(parent, &new_node, node);
                Child::Node(new_node)
            }
        } else {
            let node = if let Child::Nodes(nodes) = &current {
                log::debug!(
                    "ok here we are nodes = {:?}!",
                    nodes
                        .iter()
                        .map(|n| (n.node_name(), n.node_value()))
                        .collect::<Vec<_>>()
                );
                if let Some(node) = nodes.get(0) {
                    if node.node_type() == 3 {
                        log::debug!("setting node data to {data:?}");
                        node.unchecked_ref::<web_sys::Text>().set_data(data);
                        node.clone()
                    } else {
                        create_text_node(data).unchecked_into()
                    }
                } else {
                    create_text_node(data).unchecked_into()
                }
            } else {
                create_text_node(data).unchecked_into()
            };
            clean_children(parent, current, before, Some(node))
        }
    } else {
        match current {
            Child::Text(_) => match parent.first_child() {
                Some(child) => {
                    child.unchecked_ref::<web_sys::Text>().set_data(data);
                }
                None => parent.set_text_content(Some(data)),
            },
            Child::Node(node) => match parent.first_child() {
                Some(child) => {
                    if let Ok(text_node) = child.dyn_into::<web_sys::Text>() {
                        text_node.set_data(data);
                    } else {
                        replace_child(parent, create_text_node(data).unchecked_ref(), &node);
                    }
                }
                None => parent.set_text_content(Some(data)),
            },
            _ => parent.set_text_content(Some(data)),
        }
        Child::Text(data.to_string())
    }
}

fn append_nodes(
    parent: &web_sys::Element,
    new_nodes: &[web_sys::Node],
    marker: &Marker,
) -> Vec<web_sys::Node> {
    let mut result = Vec::new();
    log::debug!(
        "appending nodes to {} with parent {}",
        parent.node_name(),
        parent.parent_node().unwrap().node_name()
    );
    for node in new_nodes {
        result.push(insert_before(parent, node, marker.as_some_node()));
    }
    result
}

fn clean_children(
    parent: &web_sys::Element,
    current: Child,
    marker: &Marker,
    replacement: Option<web_sys::Node>,
) -> Child {
    log::debug!("clean_children on {} with current = {current:?} and marker = {marker:#?} and replacement = {replacement:#?}", parent.node_name());

    if marker == &Marker::NoChildren {
        parent.set_text_content(Some(""));
        Child::Null
    } else {
        let mut node = replacement.unwrap_or_else(|| create_text_node("").unchecked_into());

        match current {
            Child::Null => Child::Node(insert_before(parent, &node, marker.as_some_node())),
            Child::Text(_) => Child::Node(insert_before(parent, &node, marker.as_some_node())),
            Child::Node(_) => Child::Node(insert_before(parent, &node, marker.as_some_node())),
            Child::Nodes(nodes) => {
                if nodes.is_empty() {
                    Child::Node(insert_before(parent, &node, marker.as_some_node()))
                } else {
                    let mut inserted = false;
                    log::debug!("iterating over current nodes");
                    //log::debug!("node = {} => {:?}", node.node_name(), node.node_value());
                    for (idx, el) in nodes.iter().enumerate().rev() {
                        //log::debug!("{idx}: {} => {:?}", el.node_name(), el.node_value());
                        if &node != el {
                            let is_parent =
                                el.parent_node() == Some(parent.clone().unchecked_into());
                            //log::debug!("is_parent = {is_parent}");
                            if !inserted && idx == 0 {
                                //log::debug!("!insert && idx == 0");
                                if is_parent {
                                    /* log::debug!(
                                        "replacing child {}/{:?} with {}/{:?}",
                                        el.node_name(),
                                        el.node_value(),
                                        node.node_name(),
                                        node.node_value()
                                    ); */
                                    replace_child(parent, &node, el);
                                } else {
                                    /*  log::debug!(
                                        "inserting {:?} before {:?} on {:?}",
                                        node.node_name(),
                                        marker.as_some_node().map(|n| n.node_name()),
                                        parent.outer_html()
                                    ); */
                                    node = insert_before(parent, &node, marker.as_some_node());
                                }
                            } else {
                                el.unchecked_ref::<web_sys::Element>().remove();
                            }
                        } else {
                            inserted = true;
                        }
                    }
                    Child::Node(node)
                }
            }
            Child::Fn(_) => todo!(),
        }
    }
}
