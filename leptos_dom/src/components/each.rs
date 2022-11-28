#[cfg(all(target_arch = "wasm32", feature = "web"))]
use crate::{mount_child, GetWebSysNode, MountKind};
use crate::{Comment, CoreComponent, IntoNode, Node};
use leptos_reactive::{create_effect, Scope};
use smallvec::SmallVec;
use std::{
  borrow::Cow, cell::RefCell, collections::HashSet, hash::Hash, rc::Rc,
};
use wasm_bindgen::JsCast;

#[cfg(all(target_arch = "wasm32", feature = "web"))]
trait VecExt {
  fn get_next_closest_mounted_sibling(
    &self,
    start_at: usize,
    or: web_sys::Node,
  ) -> web_sys::Node;
}

#[cfg(all(target_arch = "wasm32", feature = "web"))]
impl VecExt for Vec<EachItem> {
  fn get_next_closest_mounted_sibling(
    &self,
    start_at: usize,
    or: web_sys::Node,
  ) -> web_sys::Node {
    self[start_at..]
      .iter()
      .find_map(|s| s.child.is_some().then_some(s.opening.node.clone()))
      .unwrap_or(or)
  }
}

/// The internal representation of the [`Each`] core-component.
#[derive(Debug)]
pub struct EachRepr {
  #[cfg(all(target_arch = "wasm32", feature = "web"))]
  document_fragment: web_sys::DocumentFragment,
  #[cfg(debug_assertions)]
  opening: Comment,
  children: Rc<RefCell<Vec<EachItem>>>,
  closing: Comment,
}

impl Default for EachRepr {
  fn default() -> Self {
    let (opening, closing) = {
      let (opening, closing) = (
        Comment::new(Cow::Borrowed("<Each>")),
        Comment::new(Cow::Borrowed("</Each>")),
      );

      (opening, closing)
    };

    #[cfg(all(target_arch = "wasm32", feature = "web"))]
    let document_fragment = {
      let fragment = crate::document().create_document_fragment();

      // Insert the comments into the document fragment
      // so they can serve as our references when inserting
      // future nodes
      #[cfg(debug_assertions)]
      fragment
        .append_with_node_2(&opening.node, &closing.node)
        .expect("append to not err");

      #[cfg(not(debug_assertions))]
      fragment
        .append_with_node_1(&closing.node)
        .expect("append to not err");

      fragment
    };

    Self {
      #[cfg(all(target_arch = "wasm32", feature = "web"))]
      document_fragment,
      //#[cfg(debug_assertions)]
      opening,
      children: Default::default(),
      closing,
    }
  }
}

impl EachRepr {
  #[cfg(all(target_arch = "wasm32", feature = "web"))]
  pub(crate) fn get_web_sys_node(&self) -> web_sys::Node {
    self.document_fragment.clone().unchecked_into()
  }
}

/// The internal representation of an [`Each`] item.
#[derive(Debug)]
struct EachItem {
  #[cfg(all(target_arch = "wasm32", feature = "web"))]
  document_fragment: web_sys::DocumentFragment,
  opening: Comment,
  child: Option<Node>,
  closing: Comment,
}

impl Default for EachItem {
  fn default() -> Self {
    let (opening, closing) = {
      let (opening, closing) = (
        Comment::new(Cow::Borrowed("<EachItem>")),
        Comment::new(Cow::Borrowed("</EachItem>")),
      );

      (opening, closing)
    };

    #[cfg(all(target_arch = "wasm32", feature = "web"))]
    let document_fragment = {
      let fragment = crate::document().create_document_fragment();

      // Insert the comments into the document fragment
      // so they can serve as our references when inserting
      // future nodes
      #[cfg(debug_assertions)]
      fragment
        .append_with_node_2(&opening.node, &closing.node)
        .expect("append to not err");

      fragment
    };

    Self {
      #[cfg(all(target_arch = "wasm32", feature = "web"))]
      document_fragment,
      opening,
      child: Default::default(),
      closing,
    }
  }
}

#[cfg(all(target_arch = "wasm32", feature = "web"))]
impl GetWebSysNode for EachItem {
  fn get_web_sys_node(&self) -> web_sys::Node {
    self.document_fragment.clone().unchecked_into()
  }
}

impl EachItem {
  /// Moves all child nodes into its' `DocumentFragment` in
  /// order to be reinserted somewhere else.
  #[cfg(all(target_arch = "wasm32", feature = "web"))]
  fn prepare_for_move(&self) {
    let start = &self.opening.node;
    let end = &self.closing.node;

    let r = web_sys::Range::new().unwrap();

    r.set_start_before(start).unwrap();
    r.set_end_after(end).unwrap();

    let frag = r.extract_contents().unwrap();

    self.document_fragment.append_child(&frag).unwrap();
  }
}

#[derive(typed_builder::TypedBuilder)]
struct EachProps {}

/// A component for efficiently rendering an iterable.
pub struct EachKey<IF, I, T, EF, N, KF, K>
where
  IF: Fn() -> I + 'static,
  I: IntoIterator<Item = T>,
  EF: Fn(T) -> N + 'static,
  N: IntoNode,
  KF: Fn(&T) -> K + 'static,
  K: Eq + Hash + 'static,
  T: 'static,
{
  items_fn: IF,
  each_fn: EF,
  key_fn: KF,
}

impl<IF, I, T, EF, N, KF, K> EachKey<IF, I, T, EF, N, KF, K>
where
  IF: Fn() -> I + 'static,
  I: IntoIterator<Item = T>,
  EF: Fn(T) -> N + 'static,
  N: IntoNode,
  KF: Fn(&T) -> K,
  K: Eq + Hash + 'static,
  T: 'static,
{
  /// Creates a new [`Each`] component.
  pub fn new(items_fn: IF, key_fn: KF, each_fn: EF) -> Self {
    Self {
      items_fn,
      each_fn,
      key_fn,
    }
  }
}

impl<IF, I, T, EF, N, KF, K> IntoNode for EachKey<IF, I, T, EF, N, KF, K>
where
  IF: Fn() -> I + 'static,
  I: IntoIterator<Item = T>,
  EF: Fn(T) -> N + 'static,
  N: IntoNode,
  KF: Fn(&T) -> K + 'static,
  K: Eq + Hash + 'static,
  T: 'static,
{
  #[instrument(level = "trace", name = "<Each />", skip_all)]
  fn into_node(self, cx: leptos_reactive::Scope) -> crate::Node {
    let Self {
      items_fn,
      each_fn,
      key_fn,
    } = self;

    let component = EachRepr::default();

    let children = component.children.clone();

    #[cfg(all(target_arch = "wasm32", feature = "web"))]
    let (opening, closing) = (
      component.opening.node.clone(),
      component.closing.node.clone(),
    );

    create_effect(cx, move |prev_hash_run| {
      let items = items_fn();

      let items = items.into_iter().collect::<Vec<_>>();

      let hashed_items = items
        .iter()
        .enumerate()
        .map(|(idx, i)| HashKey(key_fn(i), idx))
        .collect::<HashSet<_, _>>();

      if let Some(HashRun(prev_hash_run)) = prev_hash_run {
        let cmds = diff(&prev_hash_run, &hashed_items);

        debug!(diff = ?cmds);

        apply_cmds(
          cx,
          #[cfg(all(target_arch = "wasm32", feature = "web"))]
          &opening,
          &closing,
          cmds,
          &mut children.borrow_mut(),
          items.into_iter().map(|t| Some(t)).collect(),
          &each_fn,
        );
      } else {
        let mut children_borrow = children.borrow_mut();

        *children_borrow = Vec::with_capacity(items.len());

        for item in items {
          let mut each_item = EachItem::default();

          let child = each_fn(item).into_node(cx);

          #[cfg(all(target_arch = "wasm32", feature = "web"))]
          mount_child(MountKind::Component(&each_item.closing.node), &child);

          each_item.child = Some(child);

          #[cfg(all(target_arch = "wasm32", feature = "web"))]
          mount_child(MountKind::Component(&closing), &each_item);

          children_borrow.push(each_item);
        }
      }

      HashRun(hashed_items)
    });

    Node::CoreComponent(CoreComponent::Each(component))
  }
}

#[derive(educe::Educe)]
#[educe(Debug)]
struct HashRun<T>(#[educe(Debug(ignore))] T);

/// Calculates the operations need to get from `a` to `b`.
fn diff<K: Eq + Hash>(
  from: &HashSet<HashKey<K>>,
  to: &HashSet<HashKey<K>>,
) -> Diff {
  let mut cmds = Vec::with_capacity(to.len());

  // Get removed items
  let removed = from.difference(to).map(|k| DiffOp::Remove { at: k.1 });

  // Get added items
  let added = to.difference(from).map(|k| DiffOp::Add { at: k.1 });

  // Get maybe moved items
  let moved = from
    .intersection(to)
    .map(|k| (from.get(k).unwrap().1, to.get(k).unwrap().1))
    .filter(|(from, to)| from != to)
    .map(|(from, to)| DiffOp::Move { from, to });

  cmds.extend(removed);
  let removed_amount = cmds.len();

  cmds.extend(moved);
  let moved_amount = cmds.len() - removed_amount;

  cmds.extend(added);
  let added_amount = cmds.len() - moved_amount - removed_amount;
  let delta = (added_amount as isize) - (removed_amount as isize);

  if cmds.is_empty() {
    cmds.push(DiffOp::Clear);
  }

  Diff {
    added_delta: delta,
    moving: moved_amount,
    removing: removed_amount,
    ops: cmds,
  }
}

struct HashKey<K>(K, usize);

impl<K: Hash> Hash for HashKey<K> {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    self.0.hash(state);
  }
}

impl<K: Eq> PartialEq for HashKey<K> {
  fn eq(&self, other: &Self) -> bool {
    self.0 == other.0
  }
}

impl<K: Eq> Eq for HashKey<K> {}

#[derive(Debug)]
struct Diff {
  /// The number of items added minus the number of items removed, used
  /// for optimizing reallocations for children.
  added_delta: isize,
  /// Number of items that will need to be moved.
  moving: usize,
  /// Number of items that will be removed.
  removing: usize,
  ops: Vec<DiffOp>,
}

#[derive(Debug)]
enum DiffOp {
  Move { from: usize, to: usize },
  Add { at: usize },
  Remove { at: usize },
  Clear,
}

fn apply_cmds<T, EF, N>(
  cx: Scope,
  #[cfg(all(target_arch = "wasm32", feature = "web"))] opening: &web_sys::Node,
  #[cfg(all(target_arch = "wasm32", feature = "web"))] closing: &web_sys::Node,
  mut cmds: Diff,
  children: &mut Vec<EachItem>,
  mut items: Vec<Option<T>>,
  each_fn: &EF,
) where
  EF: Fn(T) -> N,
  N: IntoNode,
{
  // Resize children if needed
  if cmds.added_delta >= 0 {
    children.resize_with(children.len() + cmds.added_delta as usize, || {
      EachItem::default()
    });
  }

  // We need to hold a list of items which will be moved, and
  // we can only perform the omve after all commands have run, otherwise,
  // we risk overwriting one of the values
  let mut items_to_move = Vec::with_capacity(cmds.moving);

  // We can optimize the case of replacing all items
  if cmds.removing == children.len() {
    children.clear();

    #[cfg(all(target_arch = "wasm32", feature = "web"))]
    {
      let range = web_sys::Range::new().unwrap();

      range.set_start_after(opening).unwrap();
      range.set_end_before(opening).unwrap();

      range.delete_contents().unwrap();
    }

    cmds
      .ops
      .drain_filter(|cmd| !matches!(cmd, DiffOp::Add { .. }));
  }

  // The order of cmds needs to be:
  // 1. Removed
  // 2. Moved
  // 3. Add
  for cmd in cmds.ops {
    match cmd {
      DiffOp::Remove { at } => {
        let item_to_remove = std::mem::take(&mut children[at]);

        #[cfg(all(target_arch = "wasm32", feature = "web"))]
        {
          let range = web_sys::Range::new().unwrap();

          range
            .set_start_before(&item_to_remove.opening.node)
            .unwrap();
          range.set_end_after(&item_to_remove.closing.node).unwrap();

          range.delete_contents().unwrap();
        }
      }
      DiffOp::Move { from, to } => {
        let item = std::mem::take(&mut children[from]);

        #[cfg(all(target_arch = "wasm32", feature = "web"))]
        item.prepare_for_move();

        items_to_move.push((to, item));
      }
      DiffOp::Add { at } => {
        let item = std::mem::replace(&mut items[at], None).unwrap();

        let child = each_fn(item).into_node(cx);

        let mut each_item = EachItem::default();

        #[cfg(all(target_arch = "wasm32", feature = "web"))]
        mount_child(MountKind::Component(&each_item.closing.node), &child);

        each_item.child = Some(child);

        #[cfg(all(target_arch = "wasm32", feature = "web"))]
        {
          let opening = children
            .get_next_closest_mounted_sibling(at + 1, closing.to_owned());

          mount_child(MountKind::Component(&opening), &each_item);
        }

        children[at] = each_item;
      }
      DiffOp::Clear => {
        children.clear();

        #[cfg(all(target_arch = "wasm32", feature = "web"))]
        {
          let range = web_sys::Range::new().unwrap();

          range.set_start_after(opening).unwrap();
          range.set_end_before(closing).unwrap();

          range.delete_contents().unwrap();
        }
      }
    }
  }

  for (to, each_item) in items_to_move {
    #[cfg(all(target_arch = "wasm32", feature = "web"))]
    {
      let opening =
        children.get_next_closest_mounted_sibling(to + 1, closing.to_owned());

      mount_child(MountKind::Component(&opening), &each_item);

      children[to] = each_item;
    }
  }
}
