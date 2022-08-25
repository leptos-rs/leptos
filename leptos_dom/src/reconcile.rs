use std::collections::HashMap;

use crate::insert_before;

struct NodeWrapper<'a>(&'a web_sys::Node);

impl<'a> std::hash::Hash for NodeWrapper<'a> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::ptr::hash(self.0, state)
    }
}

impl<'a> PartialEq for NodeWrapper<'a> {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(&self.0, &other.0)
    }
}

impl<'a> Eq for NodeWrapper<'a> {}

pub fn reconcile_arrays(parent: &web_sys::Element, a: &mut [web_sys::Node], b: &[web_sys::Node]) {
    debug_assert!(!a.is_empty(), "a cannot be empty");

    // Sanity check: make sure all nodes in a are children of parent.
    #[cfg(debug_assertions)]
    {
        for (i, node) in a.iter().enumerate() {
            if node.parent_node().as_ref() != Some(parent) {
                panic!(
                    "node {} in existing nodes Vec is not a child of parent. node = {:#?}",
                    i, node
                );
            }
        }
    }

    let b_len = b.len();
    let mut a_end = a.len();
    let mut b_end = b_len;
    let mut a_start = 0;
    let mut b_start = 0;
    let mut map = None::<HashMap<NodeWrapper, usize>>;

    // Last node in a.
    let after = a[a_end - 1].next_sibling();

    while a_start < a_end || b_start < b_end {
        if a_end == a_start {
            // Append.
            let node = if b_end < b_len {
                if b_start != 0 {
                    b[b_start - 1].next_sibling()
                } else {
                    Some(b[b_end - b_start].clone())
                }
            } else {
                after.clone()
            };

            for new_node in &b[b_start..b_end] {
                insert_before(parent, new_node, node.as_ref());
            }
            b_start = b_end;
        } else if b_end == b_start {
            // Remove.
            for node in &a[a_start..a_end] {
                crate::debug_warn!("unwrap at 68");
                if map.is_none() || !map.as_ref().unwrap().contains_key(&NodeWrapper(node)) {
                    parent.remove_child(node);
                }
            }
            a_start = a_end;
        } else if a[a_start] == b[b_start] {
            // Common prefix.
            a_start += 1;
            b_start += 1;
        } else if a[a_end - 1] == b[b_end - 1] {
            // Common suffix.
            a_end -= 1;
            b_end -= 1;
        } else if a[a_start] == b[b_end - 1] && b[b_start] == a[a_end - 1] {
            // Swap backwards.
            let node = a[a_end - 1].next_sibling();
            parent.insert_before(&b[b_start], a[a_start].next_sibling().as_ref());
            parent.insert_before(&b[b_end - 1], node.as_ref());
            a_start += 1;
            b_start += 1;
            a_end -= 1;
            b_end -= 1;
            a[a_end] = b[b_end].clone();
        } else {
            // Fallback to map.
            if map.is_none() {
                let tmp = b[b_start..b_end]
                    .iter()
                    .enumerate()
                    .map(|(i, g)| (NodeWrapper(&g), i))
                    .collect();
                map = Some(tmp);
            }
            crate::debug_warn!("unwrap at 102");
            let map = map.as_ref().unwrap();

            if let Some(&index) = map.get(&NodeWrapper(&a[a_start])) {
                if b_start < index && index < b_end {
                    let mut i = a_start;
                    let mut sequence = 1;
                    let mut t;

                    while i + 1 < a_end && i + 1 < b_end {
                        i += 1;
                        t = map.get(&NodeWrapper(&a[i])).copied();
                        if t != Some(index + sequence) {
                            break;
                        }
                        sequence += 1;
                    }

                    if sequence > index - b_start {
                        let node = &a[a_start];
                        while b_start < index {
                            parent.insert_before(&b[b_start], Some(node));
                            b_start += 1;
                        }
                    } else {
                        parent.replace_child(&a[a_start], &b[b_start]);
                        a_start += 1;
                        b_start += 1;
                    }
                } else {
                    a_start += 1;
                }
            } else {
                parent.remove_child(&a[a_start]);
                a_start += 1;
            }
        }
    }

    // Sanity check: make sure all nodes in b are children of parent after reconciliation.
    #[cfg(debug_assertions)]
    {
        for (i, node) in b.iter().enumerate() {
            if node.parent_node().as_ref() != Some(parent) {
                panic!(
                    "node {} in new nodes Vec is not a child of parent after reconciliation. node = {:#?}",
                    i, node
                );
            }
        }
    }
}
