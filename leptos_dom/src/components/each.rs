#[cfg(all(target_arch = "wasm32", feature = "web"))]
use crate::{mount_child, MountKind, Mountable, RANGE};
use crate::{Comment, CoreComponent, IntoNode, Node};
use itertools::Itertools;
use leptos_reactive::{create_effect, Scope};
use rustc_hash::FxHasher;
use smallvec::{smallvec, SmallVec};
use std::{
  borrow::Cow,
  cell::RefCell,
  hash::{BuildHasherDefault, Hash},
  rc::Rc,
};
use wasm_bindgen::JsCast;

type FxIndexSet<T> = indexmap::IndexSet<T, BuildHasherDefault<FxHasher>>;

#[cfg(all(target_arch = "wasm32", feature = "web"))]
trait VecExt {
  fn get_next_closest_mounted_sibling(
    &self,
    start_at: usize,
    or: web_sys::Node,
  ) -> web_sys::Node;
}

#[cfg(all(target_arch = "wasm32", feature = "web"))]
impl VecExt for Vec<Option<EachItem>> {
  fn get_next_closest_mounted_sibling(
    &self,
    start_at: usize,
    or: web_sys::Node,
  ) -> web_sys::Node {
    self[start_at..]
      .iter()
      .find_map(|s| s.as_ref().map(|s| s.get_opening_node()))
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
  children: Rc<RefCell<Vec<Option<EachItem>>>>,
  closing: Comment,
}

impl Default for EachRepr {
  fn default() -> Self {
    let markers = (
      Comment::new(Cow::Borrowed("</Each>")),
      #[cfg(debug_assertions)]
      Comment::new(Cow::Borrowed("<Each>")),
    );

    #[cfg(all(target_arch = "wasm32", feature = "web"))]
    let document_fragment = {
      let fragment = crate::document().create_document_fragment();

      // Insert the comments into the document fragment
      #[cfg(debug_assertions)]
      // so they can serve as our references when inserting
      // future nodes
      fragment
        .append_with_node_2(&markers.1.node, &markers.0.node)
        .expect("append to not err");

      #[cfg(not(debug_assertions))]
      fragment
        .append_with_node_1(&markers.0.node)
        .expect("append to not err");

      fragment
    };

    Self {
      #[cfg(all(target_arch = "wasm32", feature = "web"))]
      document_fragment,
      #[cfg(debug_assertions)]
      opening: markers.1,
      children: Default::default(),
      closing: markers.0,
    }
  }
}

#[cfg(all(target_arch = "wasm32", feature = "web"))]
impl Mountable for EachRepr {
  fn get_mountable_node(&self) -> web_sys::Node {
    self.document_fragment.clone().unchecked_into()
  }

  fn get_opening_node(&self) -> web_sys::Node {
    #[cfg(debug_assertions)]
    return self.opening.node.clone();

    #[cfg(not(debug_assertions))]
    return {
      let children_borrow = self.children.borrow();

      if let Some(Some(child)) = children_borrow.get(0) {
        child.get_opening_node()
      } else {
        self.closing.node.clone()
      }
    };
  }
}

/// The internal representation of an [`Each`] item.
#[derive(Debug)]
struct EachItem {
  #[cfg(all(target_arch = "wasm32", feature = "web"))]
  document_fragment: web_sys::DocumentFragment,
  #[cfg(debug_assertions)]
  opening: Comment,
  child: Node,
  closing: Comment,
}

impl EachItem {
  fn new(child: Node) -> Self {
    let markers = (
      Comment::new(Cow::Borrowed("</EachItem>")),
      #[cfg(debug_assertions)]
      Comment::new(Cow::Borrowed("<EachItem>")),
    );

    #[cfg(all(target_arch = "wasm32", feature = "web"))]
    let document_fragment = {
      let fragment = crate::document().create_document_fragment();

      // Insert the comments into the document fragment
      // so they can serve as our references when inserting
      // future nodes
      #[cfg(debug_assertions)]
      fragment
        .append_with_node_2(&markers.1.node, &markers.0.node)
        .unwrap();
      fragment.append_with_node_1(&markers.0.node).unwrap();

      mount_child(MountKind::Before(&markers.0.node), &child);

      fragment
    };

    Self {
      #[cfg(all(target_arch = "wasm32", feature = "web"))]
      document_fragment,
      #[cfg(debug_assertions)]
      opening: markers.1,
      child,
      closing: markers.0,
    }
  }
}

#[cfg(all(target_arch = "wasm32", feature = "web"))]
impl Mountable for EachItem {
  fn get_mountable_node(&self) -> web_sys::Node {
    self.document_fragment.clone().unchecked_into()
  }

  fn get_opening_node(&self) -> web_sys::Node {
    #[cfg(debug_assertions)]
    return self.opening.node.clone();

    #[cfg(not(debug_assertions))]
    return self.child.get_opening_node();
  }
}

impl EachItem {
  /// Moves all child nodes into its' `DocumentFragment` in
  /// order to be reinserted somewhere else.
  #[cfg(all(target_arch = "wasm32", feature = "web"))]
  fn prepare_for_move(&self) {
    let start = self.get_opening_node();
    let end = &self.closing.node;

    let mut sibling = start;

    while sibling != *end {
      let next_sibling = sibling.next_sibling().unwrap();

      self.document_fragment.append_child(&sibling).unwrap();

      sibling = next_sibling;
    }

    self.document_fragment.append_with_node_1(end).unwrap();
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
  #[cfg_attr(
    debug_assertions,
    instrument(level = "trace", name = "<Each />", skip_all)
  )]
  fn into_node(self, cx: leptos_reactive::Scope) -> crate::Node {
    let Self {
      items_fn,
      each_fn,
      key_fn,
    } = self;

    let component = EachRepr::default();

    let children = component.children.clone();

    #[cfg(all(target_arch = "wasm32", feature = "web"))]
    let closing = component.closing.node.clone();

    create_effect(cx, move |prev_hash_run| {
      let children_borrow = children.borrow();

      #[cfg(all(target_arch = "wasm32", feature = "web"))]
      let opening = if let Some(Some(child)) = children_borrow.get(0) {
        child.get_opening_node()
      } else {
        closing.clone()
      };

      drop(children_borrow);

      let items = items_fn();

      let items = items.into_iter().collect::<SmallVec<[_; 128]>>();

      let hashed_items = items
        .iter()
        .enumerate()
        .map(|(idx, i)| HashKey(key_fn(i), idx))
        .collect::<FxIndexSet<_>>();

      if let Some(HashRun(prev_hash_run)) = prev_hash_run {
        let cmds = diff(&prev_hash_run, &hashed_items);

        apply_cmds(
          cx,
          #[cfg(all(target_arch = "wasm32", feature = "web"))]
          &opening,
          #[cfg(all(target_arch = "wasm32", feature = "web"))]
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
          let child = each_fn(item).into_node(cx);

          let each_item = EachItem::new(child);

          #[cfg(all(target_arch = "wasm32", feature = "web"))]
          mount_child(MountKind::Before(&closing), &each_item);

          children_borrow.push(Some(each_item));
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
  from: &FxIndexSet<HashKey<K>>,
  to: &FxIndexSet<HashKey<K>>,
) -> Diff {
  if from.is_empty() && to.is_empty() {
    return Diff::default();
  } else if to.is_empty() {
    return Diff {
      ops: vec![DiffOp::Clear],
      ..Default::default()
    };
  }

  let mut cmds = Vec::with_capacity(to.len());

  // Get removed items
  let mut removed = from.difference(to);

  let removed_cmds = removed.clone().map(|k| DiffOp::Remove { at: k.1 });

  // Get added items
  let mut added = to.difference(from);

  let added_cmds = added.clone().map(|k| DiffOp::Add {
    at: k.1,
    mode: Default::default(),
  });

  // Get moved items
  //
  // Todo: fix this move...we need to calculate to see if the value actually
  // moved, as in, if we account for all insertions, and all deletions, are
  // we left with a value that actually moved?
  let moved = from
    .intersection(to)
    .map(|k| (from.get(k).unwrap().1, to.get(k).unwrap().1))
    .filter(|(from, to)| from != to)
    .map(|(from, to)| DiffOp::Move { from, to });

  cmds.extend(removed_cmds);
  let removed_amount = cmds.len();

  cmds.extend(moved);
  let moved_amount = cmds.len() - removed_amount;

  cmds.extend(added_cmds);
  let added_amount = cmds.len() - moved_amount - removed_amount;
  let delta = (added_amount as isize) - (removed_amount as isize);

  let mut diffs = Diff {
    added_delta: delta,
    moving: moved_amount,
    removing: removed_amount,
    ops: cmds,
  };

  apply_opts(from, to, &mut diffs);

  diffs
}

fn apply_opts<K: Eq + Hash>(
  from: &FxIndexSet<HashKey<K>>,
  to: &FxIndexSet<HashKey<K>>,
  cmds: &mut Diff,
) {
  // We can optimize the case of replacing all items
  if !to.is_empty() && cmds.removing == from.len() && cmds.moving == 0 {
    cmds
      .ops
      .drain_filter(|cmd| !matches!(cmd, DiffOp::Add { .. }));

    cmds.ops.iter_mut().for_each(|op| {
      if let DiffOp::Add { mode, .. } = op {
        *mode = DiffOpAddMode::Append;
      } else {
        unreachable!()
      }
    });

    cmds.ops.insert(0, DiffOp::Clear);

    return;
  }

  // We can optimize for the case where we are only appending
  // items
  if cmds.added_delta != 0 && cmds.removing == 0 && cmds.moving == 0 {
    cmds.ops.iter_mut().for_each(|op| {
      if let DiffOp::Add { at, mode } = op {
        *mode = DiffOpAddMode::Append;
      } else {
        unreachable!()
      }
    });

    return;
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

#[derive(Debug, Default)]
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
  Add { at: usize, mode: DiffOpAddMode },
  Remove { at: usize },
  Clear,
}

#[derive(Default, Debug)]
enum DiffOpAddMode {
  #[default]
  Normal,
  Append,
  // Todo
  _Prepend,
}

fn apply_cmds<T, EF, N>(
  cx: Scope,
  #[cfg(all(target_arch = "wasm32", feature = "web"))] opening: &web_sys::Node,
  #[cfg(all(target_arch = "wasm32", feature = "web"))] closing: &web_sys::Node,
  mut cmds: Diff,
  children: &mut Vec<Option<EachItem>>,
  mut items: SmallVec<[Option<T>; 128]>,
  each_fn: &EF,
) where
  EF: Fn(T) -> N,
  N: IntoNode,
{
  #[cfg(all(target_arch = "wasm32", feature = "web"))]
  let range = &RANGE;

  // Resize children if needed
  if cmds.added_delta >= 0 {
    children.resize_with(children.len() + cmds.added_delta as usize, || None);
  }

  // We need to hold a list of items which will be moved, and
  // we can only perform the omve after all commands have run, otherwise,
  // we risk overwriting one of the values
  let mut items_to_move = Vec::with_capacity(cmds.moving);

  // The order of cmds needs to be:
  // 1. Removed
  // 2. Moved
  // 3. Add
  for cmd in cmds.ops {
    match cmd {
      DiffOp::Remove { at } => {
        let item_to_remove = std::mem::take(&mut children[at]).unwrap();

        #[cfg(all(target_arch = "wasm32", feature = "web"))]
        item_to_remove.prepare_for_move();
      }
      DiffOp::Move { from, to } => {
        let item = std::mem::take(&mut children[from]).unwrap();

        #[cfg(all(target_arch = "wasm32", feature = "web"))]
        item.prepare_for_move();

        items_to_move.push((to, item));
      }
      DiffOp::Add { at, mode } => {
        let item = std::mem::replace(&mut items[at], None).unwrap();

        let child = each_fn(item).into_node(cx);

        let each_item = EachItem::new(child);

        #[cfg(all(target_arch = "wasm32", feature = "web"))]
        {
          match mode {
            DiffOpAddMode::Normal => {
              let opening = children
                .get_next_closest_mounted_sibling(at + 1, closing.to_owned());

              mount_child(MountKind::Before(&opening), &each_item);
            }
            DiffOpAddMode::Append => {
              mount_child(MountKind::Before(closing), &each_item);
            }
            DiffOpAddMode::_Prepend => todo!(),
          }
        }

        children[at] = Some(each_item);
      }
      DiffOp::Clear => {
        #[cfg(all(target_arch = "wasm32", feature = "web"))]
        {
          // if opening.previous_sibling().is_none()
          //   && closing.next_sibling().is_none()
          // {
          //   let parent = closing
          //     .parent_node()
          //     .unwrap()
          //     .unchecked_into::<web_sys::Element>();
          //   parent.set_text_content(Some(""));

          //   #[cfg(debug_assertions)]
          //   parent.append_with_node_2(opening, closing).unwrap();

          //   #[cfg(not(debug_assertions))]
          //   parent.append_with_node_1(closing).unwrap();
          // } else {
          range.set_start_before(opening).unwrap();
          range.set_end_before(closing).unwrap();

          range.delete_contents().unwrap();
          // }
        }
      }
    }
  }

  for (to, each_item) in items_to_move {
    #[cfg(all(target_arch = "wasm32", feature = "web"))]
    {
      let opening =
        children.get_next_closest_mounted_sibling(to + 1, closing.to_owned());

      mount_child(MountKind::Before(&opening), &each_item);

      children[to] = Some(each_item);
    }
  }
}
