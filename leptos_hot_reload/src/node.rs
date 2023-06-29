use crate::parsing::is_component_node;
use anyhow::Result;
use quote::ToTokens;
use rstml::node::{Node, NodeAttribute};
use serde::{Deserialize, Serialize};

// A lightweight virtual DOM structure we can use to hold
// the state of a Leptos view macro template. This is because
// `syn` types are `!Send` so we can't store them as we might like.
// This is only used to diff view macros for hot reloading so it's very minimal
// and ignores many of the data types.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LNode {
    Fragment(Vec<LNode>),
    Text(String),
    Element {
        name: String,
        attrs: Vec<(String, LAttributeValue)>,
        children: Vec<LNode>,
    },
    // don't need anything; skipped during patching because it should
    // contain its own view macros
    Component {
        name: String,
        props: Vec<(String, String)>,
        children: Vec<LNode>,
    },
    DynChild(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LAttributeValue {
    Boolean,
    Static(String),
    // safely ignored
    Dynamic,
    Noop,
}

impl LNode {
    pub fn parse_view(nodes: Vec<Node>) -> Result<LNode> {
        let mut out = Vec::new();
        for node in nodes {
            LNode::parse_node(node, &mut out)?;
        }
        if out.len() == 1 {
            Ok(out.pop().unwrap())
        } else {
            Ok(LNode::Fragment(out))
        }
    }

    pub fn parse_node(node: Node, views: &mut Vec<LNode>) -> Result<()> {
        match node {
            Node::Fragment(frag) => {
                for child in frag.children {
                    LNode::parse_node(child, views)?;
                }
            }
            Node::Text(text) => {
                views.push(LNode::Text(text.value_string()));
            }
            Node::Block(block) => {
                let code = block.into_token_stream();
                let code = code.to_string();
                views.push(LNode::DynChild(code));
            }
            Node::Element(el) => {
                if is_component_node(&el) {
                    let name = el.name().to_string();
                    let mut children = Vec::new();
                    for child in el.children {
                        LNode::parse_node(child, &mut children)?;
                    }
                    views.push(LNode::Component {
                        name,
                        props: el
                            .open_tag
                            .attributes
                            .into_iter()
                            .filter_map(|attr| match attr {
                                NodeAttribute::Attribute(attr) => Some((
                                    attr.key.to_string(),
                                    format!("{:#?}", attr.value()),
                                )),
                                _ => None,
                            })
                            .collect(),
                        children,
                    });
                } else {
                    let name = el.name().to_string();
                    let mut attrs = Vec::new();

                    for attr in el.open_tag.attributes {
                        if let NodeAttribute::Attribute(attr) = attr {
                            let name = attr.key.to_string();
                            if let Some(value) = attr.value_literal_string() {
                                attrs.push((
                                    name,
                                    LAttributeValue::Static(value),
                                ));
                            } else {
                                attrs.push((name, LAttributeValue::Dynamic));
                            }
                        }
                    }

                    let mut children = Vec::new();
                    for child in el.children {
                        LNode::parse_node(child, &mut children)?;
                    }

                    views.push(LNode::Element {
                        name,
                        attrs,
                        children,
                    });
                }
            }
            _ => {}
        }
        Ok(())
    }

    pub fn to_html(&self) -> String {
        match self {
            LNode::Fragment(frag) => frag.iter().map(LNode::to_html).collect(),
            LNode::Text(text) => text.to_owned(),
            LNode::Component { name, .. } => format!(
                "<!--<{name}>--><pre>&lt;{name}/&gt; will load once Rust code \
                 has been compiled.</pre><!--</{name}>-->"
            ),
            LNode::DynChild(_) => "<!--<DynChild>--><pre>Dynamic content will \
                                   load once Rust code has been \
                                   compiled.</pre><!--</DynChild>-->"
                .to_string(),
            LNode::Element {
                name,
                attrs,
                children,
            } => {
                // this is naughty, but the browsers are tough and can handle it
                // I wouldn't do this for real code, but this is just for dev mode
                let is_self_closing = children.is_empty();

                let attrs = attrs
                    .iter()
                    .filter_map(|(name, value)| match value {
                        LAttributeValue::Boolean => Some(format!("{name} ")),
                        LAttributeValue::Static(value) => {
                            Some(format!("{name}=\"{value}\" "))
                        }
                        LAttributeValue::Dynamic => None,
                        LAttributeValue::Noop => None,
                    })
                    .collect::<String>();

                let children =
                    children.iter().map(LNode::to_html).collect::<String>();

                if is_self_closing {
                    format!("<{name} {attrs}/>")
                } else {
                    format!("<{name} {attrs}>{children}</{name}>")
                }
            }
        }
    }
}
