use crate::node::{LAttributeValue, LNode};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default)]
struct OldChildren(IndexMap<LNode, Vec<usize>>);

impl LNode {
    pub fn diff(&self, other: &LNode) -> Vec<Patch> {
        let mut old_children = OldChildren::default();
        self.add_old_children(vec![], &mut old_children);
        self.diff_at(other, &[], &old_children)
    }

    fn to_replacement_node(
        &self,
        old_children: &OldChildren,
    ) -> ReplacementNode {
        match old_children.0.get(self) {
            // if the child already exists in the DOM, we can pluck it out
            // and move it around
            Some(path) => ReplacementNode::Path(path.to_owned()),
            // otherwise, we should generate some HTML
            // but we need to do this recursively in case we're replacing an element
            // with children who need to be plucked out
            None => match self {
                LNode::Fragment(fragment) => ReplacementNode::Fragment(
                    fragment
                        .iter()
                        .map(|node| node.to_replacement_node(old_children))
                        .collect(),
                ),
                LNode::Element {
                    name,
                    attrs,
                    children,
                } => ReplacementNode::Element {
                    name: name.to_owned(),
                    attrs: attrs
                        .iter()
                        .filter_map(|(name, value)| match value {
                            LAttributeValue::Boolean => {
                                Some((name.to_owned(), name.to_owned()))
                            }
                            LAttributeValue::Static(value) => {
                                Some((name.to_owned(), value.to_owned()))
                            }
                            _ => None,
                        })
                        .collect(),
                    children: children
                        .iter()
                        .map(|node| node.to_replacement_node(old_children))
                        .collect(),
                },
                LNode::Text(_)
                | LNode::Component { .. }
                | LNode::DynChild(_) => ReplacementNode::Html(self.to_html()),
            },
        }
    }

    fn add_old_children(&self, path: Vec<usize>, positions: &mut OldChildren) {
        match self {
            LNode::Fragment(frag) => {
                for (idx, child) in frag.iter().enumerate() {
                    let mut new_path = path.clone();
                    new_path.push(idx);
                    child.add_old_children(new_path, positions);
                }
            }
            LNode::Element { children, .. } => {
                for (idx, child) in children.iter().enumerate() {
                    let mut new_path = path.clone();
                    new_path.push(idx);
                    child.add_old_children(new_path, positions);
                }
            }
            // need to insert dynamic content and children, as these might change
            LNode::DynChild(_) => {
                positions.0.insert(self.clone(), path);
            }
            LNode::Component { children, .. } => {
                positions.0.insert(self.clone(), path.clone());

                for (idx, child) in children.iter().enumerate() {
                    let mut new_path = path.clone();
                    new_path.push(idx);
                    child.add_old_children(new_path, positions);
                }
            }
            // can just create text nodes, whatever
            LNode::Text(_) => {}
        }
    }

    fn diff_at(
        &self,
        other: &LNode,
        path: &[usize],
        orig_children: &OldChildren,
    ) -> Vec<Patch> {
        if std::mem::discriminant(self) != std::mem::discriminant(other) {
            return vec![Patch {
                path: path.to_owned(),
                action: PatchAction::ReplaceWith(
                    other.to_replacement_node(orig_children),
                ),
            }];
        }
        match (self, other) {
            // fragment: diff children
            (LNode::Fragment(old), LNode::Fragment(new)) => {
                LNode::diff_children(path, old, new, orig_children)
            }
            // text node: replace text
            (LNode::Text(_), LNode::Text(new)) => vec![Patch {
                path: path.to_owned(),
                action: PatchAction::SetText(new.to_owned()),
            }],
            // elements
            (
                LNode::Element {
                    name: old_name,
                    attrs: old_attrs,
                    children: old_children,
                },
                LNode::Element {
                    name: new_name,
                    attrs: new_attrs,
                    children: new_children,
                },
            ) => {
                let tag_patch = (old_name != new_name).then(|| Patch {
                    path: path.to_owned(),
                    action: PatchAction::ChangeTagName(new_name.to_owned()),
                });

                let attrs_patch = LNode::diff_attrs(path, old_attrs, new_attrs);

                let children_patch = LNode::diff_children(
                    path,
                    old_children,
                    new_children,
                    orig_children,
                );

                attrs_patch
                    .into_iter()
                    // tag patch comes second so we remove old attrs before copying them over
                    .chain(tag_patch)
                    .chain(children_patch)
                    .collect()
            }
            // components + dynamic context: no patches
            (
                LNode::Component {
                    name: old_name,
                    children: old_children,
                    ..
                },
                LNode::Component {
                    name: new_name,
                    children: new_children,
                    ..
                },
            ) if old_name == new_name => {
                let mut path = path.to_vec();
                path.push(0);
                path.push(0);
                LNode::diff_children(
                    &path,
                    old_children,
                    new_children,
                    orig_children,
                )
            }
            _ => vec![],
        }
    }

    fn diff_attrs<'a>(
        path: &'a [usize],
        old: &'a [(String, LAttributeValue)],
        new: &'a [(String, LAttributeValue)],
    ) -> impl Iterator<Item = Patch> + 'a {
        let additions = new
            .iter()
            .filter_map(|(name, new_value)| {
                let old_attr = old.iter().find(|(o_name, _)| o_name == name);
                let replace = match old_attr {
                    None => true,
                    Some((_, old_value)) if old_value != new_value => true,
                    _ => false,
                };
                if replace {
                    match &new_value {
                        LAttributeValue::Boolean => {
                            Some((name.to_owned(), "".to_string()))
                        }
                        LAttributeValue::Static(s) => {
                            Some((name.to_owned(), s.to_owned()))
                        }
                        _ => None,
                    }
                } else {
                    None
                }
            })
            .map(|(name, value)| Patch {
                path: path.to_owned(),
                action: PatchAction::SetAttribute(name, value),
            });

        let removals = old.iter().filter_map(|(name, _)| {
            if !new.iter().any(|(new_name, _)| new_name == name) {
                Some(Patch {
                    path: path.to_owned(),
                    action: PatchAction::RemoveAttribute(name.to_owned()),
                })
            } else {
                None
            }
        });

        additions.chain(removals)
    }

    fn diff_children(
        path: &[usize],
        old: &[LNode],
        new: &[LNode],
        old_children: &OldChildren,
    ) -> Vec<Patch> {
        if old.is_empty() && new.is_empty() {
            vec![]
        } else if old.is_empty() {
            vec![Patch {
                path: path.to_owned(),
                action: PatchAction::AppendChildren(
                    new.iter()
                        .map(LNode::to_html)
                        .map(ReplacementNode::Html)
                        .collect(),
                ),
            }]
        } else if new.is_empty() {
            vec![Patch {
                path: path.to_owned(),
                action: PatchAction::ClearChildren,
            }]
        } else {
            let mut a = 0;
            let mut b = std::cmp::max(old.len(), new.len()) - 1; // min is 0, have checked both have items
            let mut patches = vec![];
            // common prefix
            while a < b {
                let old = old.get(a);
                let new = new.get(a);

                match (old, new) {
                    (None, None) => {}
                    (None, Some(new)) => patches.push(Patch {
                        path: path.to_owned(),
                        action: PatchAction::InsertChild {
                            before: a,
                            child: new.to_replacement_node(old_children),
                        },
                    }),
                    (Some(_), None) => patches.push(Patch {
                        path: path.to_owned(),
                        action: PatchAction::RemoveChild { at: a },
                    }),
                    (Some(old), Some(new)) => {
                        if old != new {
                            break;
                        }
                    }
                }

                a += 1;
            }

            // common suffix
            while b >= a {
                let old = old.get(b);
                let new = new.get(b);

                match (old, new) {
                    (None, None) => {}
                    (None, Some(new)) => patches.push(Patch {
                        path: path.to_owned(),
                        action: PatchAction::InsertChildAfter {
                            after: b - 1,
                            child: new.to_replacement_node(old_children),
                        },
                    }),
                    (Some(_), None) => patches.push(Patch {
                        path: path.to_owned(),
                        action: PatchAction::RemoveChild { at: b },
                    }),
                    (Some(old), Some(new)) => {
                        if old != new {
                            break;
                        }
                    }
                }

                if b == 0 {
                    break;
                } else {
                    b -= 1;
                }
            }

            // diffing in middle
            if b >= a {
                let old_slice_end =
                    if b >= old.len() { old.len() - 1 } else { b };
                let new_slice_end =
                    if b >= new.len() { new.len() - 1 } else { b };
                let old = &old[a..=old_slice_end];
                let new = &new[a..=new_slice_end];

                for (new_idx, new_node) in new.iter().enumerate() {
                    match old.get(new_idx) {
                        Some(old_node) => {
                            let mut new_path = path.to_vec();
                            new_path.push(new_idx + a);
                            let diffs = old_node.diff_at(
                                new_node,
                                &new_path,
                                old_children,
                            );
                            patches.extend(&mut diffs.into_iter());
                        }
                        None => patches.push(Patch {
                            path: path.to_owned(),
                            action: PatchAction::InsertChild {
                                before: new_idx,
                                child: new_node
                                    .to_replacement_node(old_children),
                            },
                        }),
                    }
                }
            }

            patches
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Patches(pub Vec<(String, Vec<Patch>)>);

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Patch {
    path: Vec<usize>,
    action: PatchAction,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PatchAction {
    ReplaceWith(ReplacementNode),
    ChangeTagName(String),
    RemoveAttribute(String),
    SetAttribute(String, String),
    SetText(String),
    ClearChildren,
    AppendChildren(Vec<ReplacementNode>),
    RemoveChild {
        at: usize,
    },
    InsertChild {
        before: usize,
        child: ReplacementNode,
    },
    InsertChildAfter {
        after: usize,
        child: ReplacementNode,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReplacementNode {
    Html(String),
    Path(Vec<usize>),
    Fragment(Vec<ReplacementNode>),
    Element {
        name: String,
        attrs: Vec<(String, String)>,
        children: Vec<ReplacementNode>,
    },
}

#[cfg(test)]
mod tests {
    use crate::{
        diff::{Patch, PatchAction, ReplacementNode},
        node::LAttributeValue,
        LNode,
    };

    #[test]
    fn patches_text() {
        let a = LNode::Text("foo".into());
        let b = LNode::Text("bar".into());
        let delta = a.diff(&b);
        assert_eq!(
            delta,
            vec![Patch {
                path: vec![],
                action: PatchAction::SetText("bar".into())
            }]
        );
    }

    #[test]
    fn patches_attrs() {
        let a = LNode::Element {
            name: "button".into(),
            attrs: vec![
                ("class".into(), LAttributeValue::Static("a".into())),
                ("type".into(), LAttributeValue::Static("button".into())),
            ],
            children: vec![],
        };
        let b = LNode::Element {
            name: "button".into(),
            attrs: vec![
                ("class".into(), LAttributeValue::Static("a b".into())),
                ("id".into(), LAttributeValue::Static("button".into())),
            ],
            children: vec![],
        };
        let delta = a.diff(&b);
        assert_eq!(
            delta,
            vec![
                Patch {
                    path: vec![],
                    action: PatchAction::SetAttribute(
                        "class".into(),
                        "a b".into()
                    )
                },
                Patch {
                    path: vec![],
                    action: PatchAction::SetAttribute(
                        "id".into(),
                        "button".into()
                    )
                },
                Patch {
                    path: vec![],
                    action: PatchAction::RemoveAttribute("type".into())
                },
            ]
        );
    }

    #[test]
    fn patches_child_text() {
        let a = LNode::Element {
            name: "button".into(),
            attrs: vec![],
            children: vec![
                LNode::Text("foo".into()),
                LNode::Text("bar".into()),
            ],
        };
        let b = LNode::Element {
            name: "button".into(),
            attrs: vec![],
            children: vec![
                LNode::Text("foo".into()),
                LNode::Text("baz".into()),
            ],
        };
        let delta = a.diff(&b);
        assert_eq!(
            delta,
            vec![Patch {
                path: vec![1],
                action: PatchAction::SetText("baz".into())
            },]
        );
    }

    #[test]
    fn inserts_child() {
        let a = LNode::Element {
            name: "div".into(),
            attrs: vec![],
            children: vec![LNode::Element {
                name: "button".into(),
                attrs: vec![],
                children: vec![LNode::Text("bar".into())],
            }],
        };
        let b = LNode::Element {
            name: "div".into(),
            attrs: vec![],
            children: vec![
                LNode::Element {
                    name: "button".into(),
                    attrs: vec![],
                    children: vec![LNode::Text("foo".into())],
                },
                LNode::Element {
                    name: "button".into(),
                    attrs: vec![],
                    children: vec![LNode::Text("bar".into())],
                },
            ],
        };
        let delta = a.diff(&b);
        assert_eq!(
            delta,
            vec![
                Patch {
                    path: vec![],
                    action: PatchAction::InsertChildAfter {
                        after: 0,
                        child: ReplacementNode::Element {
                            name: "button".into(),
                            attrs: vec![],
                            children: vec![ReplacementNode::Html("bar".into())]
                        }
                    }
                },
                Patch {
                    path: vec![0, 0],
                    action: PatchAction::SetText("foo".into())
                }
            ]
        );
    }

    #[test]
    fn removes_child() {
        let a = LNode::Element {
            name: "div".into(),
            attrs: vec![],
            children: vec![
                LNode::Element {
                    name: "button".into(),
                    attrs: vec![],
                    children: vec![LNode::Text("foo".into())],
                },
                LNode::Element {
                    name: "button".into(),
                    attrs: vec![],
                    children: vec![LNode::Text("bar".into())],
                },
            ],
        };
        let b = LNode::Element {
            name: "div".into(),
            attrs: vec![],
            children: vec![LNode::Element {
                name: "button".into(),
                attrs: vec![],
                children: vec![LNode::Text("foo".into())],
            }],
        };
        let delta = a.diff(&b);
        assert_eq!(
            delta,
            vec![Patch {
                path: vec![],
                action: PatchAction::RemoveChild { at: 1 }
            },]
        );
    }
}
