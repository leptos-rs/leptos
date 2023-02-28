use crate::node::{LAttributeValue, LNode};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// TODO: insertion and removal code are still somewhat broken
// namely, it will tend to remove and move or mutate nodes,
// which causes a bit of a problem for DynChild etc.

impl LNode {
    pub fn diff(&self, other: &LNode) -> Vec<Patch> {
        self.diff_at(other, &[])
    }

    pub fn diff_at(&self, other: &LNode, path: &[usize]) -> Vec<Patch> {
        if std::mem::discriminant(self) != std::mem::discriminant(other) {
            return vec![Patch {
                path: path.to_owned(),
                action: PatchAction::ReplaceWith(other.to_html()),
            }];
        }
        match (self, other) {
            // fragment: diff children
            (LNode::Fragment(old), LNode::Fragment(new)) => {
                LNode::diff_children(path, old, new)
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
                let children_patch =
                    LNode::diff_children(path, old_children, new_children);
                tag_patch
                    .into_iter()
                    .chain(attrs_patch)
                    .chain(children_patch)
                    .collect()
            }
            // components + dynamic context: no patches
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
    ) -> Vec<Patch> {
        if old.is_empty() && new.is_empty() {
            vec![]
        } else if old.is_empty() {
            vec![Patch {
                path: path.to_owned(),
                action: PatchAction::AppendChildren(
                    new.iter().map(LNode::to_html).collect(),
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
                            child: new.to_html(),
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
                    (None, Some(new)) => { /*panic!("point B"); patches.push(Patch {
                             path: path.to_owned(),
                             action: PatchAction::AppendChild {
                                 at: b,
                                 child: new.to_owned(),
                             },
                         })*/
                    }
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

                b = b.saturating_sub(1);
            }

            // diffing in middle
            if b > a {
                let old_slice_end =
                    if b >= old.len() { old.len() - 1 } else { b };
                let new_slice_end =
                    if b >= new.len() { new.len() - 1 } else { b };
                let old = &old[a..=old_slice_end];
                let new = &new[a..=new_slice_end];

                let old_locations = old
                    .iter()
                    .enumerate()
                    .map(|(idx, node)| (node, idx))
                    .collect::<HashMap<_, _>>();
                for (new_idx, new_node) in new.iter().enumerate() {
                    match old_locations.get(new_node) {
                        Some(old_idx) if *old_idx == new_idx => {}
                        Some(old_idx) => patches.push(Patch {
                            path: path.to_owned(),
                            action: PatchAction::MoveChild {
                                from: *old_idx,
                                to: new_idx,
                            },
                        }),
                        None => match old.get(new_idx) {
                            None => patches.push(Patch {
                                path: path.to_owned(),
                                action: PatchAction::InsertChild {
                                    before: new_idx + 1,
                                    child: new_node.to_html(),
                                },
                            }),
                            Some(old_node) => {
                                let mut new_path = path.to_owned();
                                new_path.push(new_idx + a);
                                patches.extend(
                                    old_node.diff_at(new_node, &new_path),
                                );
                            }
                        },
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
    ReplaceWith(String),
    ChangeTagName(String),
    RemoveAttribute(String),
    SetAttribute(String, String),
    SetText(String),
    ClearChildren,
    AppendChildren(String),
    RemoveChild { at: usize },
    InsertChild { before: usize, child: String },
    MoveChild { from: usize, to: usize },
}

#[cfg(test)]
mod tests {
    use crate::{
        diff::{Patch, PatchAction},
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
            children: vec![LNode::Text("foo".into())],
        };
        let b = LNode::Element {
            name: "button".into(),
            attrs: vec![],
            children: vec![LNode::Text("bar".into())],
        };
        let delta = a.diff(&b);
        assert_eq!(
            delta,
            vec![Patch {
                path: vec![0],
                action: PatchAction::SetText("bar".into())
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
            vec![Patch {
                path: vec![],
                action: PatchAction::InsertChild {
                    before: 1,
                    child: "<button>foo</button>".to_string()
                }
            },]
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
