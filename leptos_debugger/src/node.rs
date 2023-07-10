use crate::{runtime::Runtime, Prop, PropValue};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug)]
pub enum DNode {
    DynChild {
        id: String,
        children: Vec<DNode>,
    },
    Unit,
    Text(String),
    Element {
        id: String,
        name: String,
        children: Vec<DNode>,
    },
    Component {
        id: String,
        name: String,
        props: Vec<Prop>,
        children: Vec<DNode>,
    },
    Root {
        signals: HashMap<u64, PropValue>,
        children: Vec<DNode>,
    },
}

pub(crate) fn create_root_tree(runtime: &Runtime) -> DNode {
    let children = runtime.nodes.borrow().get(&String::from("0-0")).map_or(
        vec![],
        |nodes| {
            nodes
                .iter()
                .map(|view| create_tree(runtime, view))
                .collect()
        },
    );

    DNode::Root {
        signals: runtime.signals.borrow().clone(),
        children,
    }
}

fn create_tree(runtime: &Runtime, node: &DNode) -> DNode {
    match node {
        DNode::Text(text) => DNode::Text(text.to_string()),
        DNode::Element { id, name, .. } => {
            let children =
                runtime.nodes.borrow().get(id).map_or(vec![], |nodes| {
                    nodes
                        .iter()
                        .map(|view| create_tree(runtime, view))
                        .collect()
                });
            DNode::Element {
                id: id.clone(),
                name: name.clone(),
                children,
            }
        }
        DNode::Component { id, name, .. } => {
            let props =
                runtime.props.borrow().get(id).map_or(vec![], |props| {
                    let mut new_props = vec![];
                    for prop in props.values() {
                        new_props.push(prop.clone());
                    }
                    new_props
                });
            let children =
                runtime.nodes.borrow().get(id).map_or(vec![], |nodes| {
                    nodes
                        .iter()
                        .map(|view| create_tree(runtime, view))
                        .collect()
                });
            DNode::Component {
                id: id.clone(),
                props,
                name: name.clone(),
                children,
            }
        }
        DNode::Root { .. } => panic!("Root should not be children"),
        DNode::Unit => DNode::Unit,
        DNode::DynChild { id, .. } => {
            let children =
                runtime.nodes.borrow().get(id).map_or(vec![], |nodes| {
                    nodes
                        .iter()
                        .map(|view| create_tree(runtime, view))
                        .collect()
                });
            DNode::DynChild {
                id: id.clone(),
                children,
            }
        }
    }
}

pub(crate) fn remove_tree(runtime: &Runtime, key: &String) {
    let props = { runtime.props.borrow_mut().remove(key) };
    if let Some(props) = props {
        for prop in props.values() {
            if let PropValue::ReadSignal(key) = prop.value {
                runtime.signals.borrow_mut().remove(&key);
            }
        }
    };

    let children = { runtime.nodes.borrow_mut().remove(key) };
    if let Some(children) = children {
        for child in children.iter() {
            match child {
                DNode::DynChild { id, .. }
                | DNode::Element { id, .. }
                | DNode::Component { id, .. } => {
                    remove_tree(runtime, id);
                }
                DNode::Unit | DNode::Text(_) => {}
                DNode::Root { .. } => panic!("Root should not be children"),
            }
        }
    }
}
