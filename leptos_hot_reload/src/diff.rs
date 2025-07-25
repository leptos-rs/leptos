use crate::node::{LAttributeValue, LNode};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default)]
struct OldChildren(IndexMap<LNode, Vec<usize>>);

impl LNode {
    #[must_use]
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
                            Some((name.to_owned(), String::new()))
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
            if new.iter().any(|(new_name, _)| new_name == name) {
                None
            } else {
                Some(Patch {
                    path: path.to_owned(),
                    action: PatchAction::RemoveAttribute(name.to_owned()),
                })
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
            let width = old.len() + 1;
            let height = new.len() + 1;
            let mut mat = vec![0; width * height];
            #[allow(clippy::needless_range_loop)]
            for i in 1..width {
                mat[i] = i;
            }
            for i in 1..height {
                mat[i * width] = i;
            }
            for j in 1..height {
                for i in 1..width {
                    if old[i - 1] == new[j - 1] {
                        mat[j * width + i] = mat[(j - 1) * width + (i - 1)];
                    } else {
                        mat[j * width + i] = (mat[(j - 1) * width + i] + 1)
                            .min(mat[j * width + (i - 1)] + 1)
                            .min(mat[(j - 1) * width + (i - 1)] + 1)
                    }
                }
            }
            let (mut i, mut j) = (old.len(), new.len());
            let mut patches = vec![];
            while i > 0 || j > 0 {
                if i > 0 && j > 0 && old[i - 1] == new[j - 1] {
                    i -= 1;
                    j -= 1;
                } else {
                    let current = mat[j * width + i];
                    if i > 0
                        && j > 0
                        && mat[(j - 1) * width + i - 1] + 1 == current
                    {
                        let mut new_path = path.to_owned();
                        new_path.push(i - 1);
                        let diffs = old[i - 1].diff_at(
                            &new[j - 1],
                            &new_path,
                            old_children,
                        );
                        patches.extend(&mut diffs.into_iter());
                        i -= 1;
                        j -= 1;
                    } else if i > 0 && mat[j * width + i - 1] + 1 == current {
                        patches.push(Patch {
                            path: path.to_owned(),
                            action: PatchAction::RemoveChild { at: i - 1 },
                        });
                        i -= 1;
                    } else if j > 0 && mat[(j - 1) * width + i] + 1 == current {
                        patches.push(Patch {
                            path: path.to_owned(),
                            action: PatchAction::InsertChild {
                                before: i,
                                child: new[j - 1]
                                    .to_replacement_node(old_children),
                            },
                        });
                        j -= 1;
                    } else {
                        unreachable!();
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
            vec![Patch {
                path: vec![],
                action: PatchAction::InsertChild {
                    before: 0,
                    child: ReplacementNode::Element {
                        name: "button".into(),
                        attrs: vec![],
                        children: vec![ReplacementNode::Html("foo".into())]
                    }
                }
            }]
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
