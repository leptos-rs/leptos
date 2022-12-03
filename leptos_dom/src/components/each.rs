#[cfg(all(target_arch = "wasm32", feature = "web"))]
use crate::{mount_child, MountKind, Mountable, RANGE};
use crate::{Comment, CoreComponent, IntoNode, Node};
use itertools::{EitherOrBoth, Itertools};
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
      fragment.append_with_node_1(&markers.0.node).unwrap();

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
      let mut children_borrow = children.borrow_mut();

      #[cfg(all(target_arch = "wasm32", feature = "web"))]
      let opening = if let Some(Some(child)) = children_borrow.get(0) {
        child.get_opening_node()
      } else {
        closing.clone()
      };

      let items = items_fn();

      let items = items.into_iter().collect::<SmallVec<[_; 128]>>();

      let hashed_items =
        items.iter().map(|i| key_fn(i)).collect::<FxIndexSet<_>>();

      if let Some(HashRun(prev_hash_run)) = prev_hash_run {
        let cmds = diff(&prev_hash_run, &hashed_items);

        apply_cmds(
          cx,
          #[cfg(all(target_arch = "wasm32", feature = "web"))]
          &opening,
          #[cfg(all(target_arch = "wasm32", feature = "web"))]
          &closing,
          cmds,
          &mut children_borrow,
          items.into_iter().map(|t| Some(t)).collect(),
          &each_fn,
        );
      } else {
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
fn diff<K: Eq + Hash>(from: &FxIndexSet<K>, to: &FxIndexSet<K>) -> Diff {
  if from.is_empty() && to.is_empty() {
    return Diff::default();
  } else if to.is_empty() {
    return Diff {
      clear: true,
      ..Default::default()
    };
  }

  // Get removed items
  let mut removed = from.difference(to);

  let removed_cmds = removed
    .clone()
    .map(|k| from.get_full(k).unwrap().0)
    .map(|idx| DiffOpRemove { at: idx });

  // Get added items
  let mut added = to.difference(from);

  let added_cmds =
    added
      .clone()
      .map(|k| to.get_full(k).unwrap().0)
      .map(|idx| DiffOpAdd {
        at: idx,
        mode: Default::default(),
      });

  // Get moved items
  let mut normalized_idx = 0;
  let mut move_cmds = SmallVec::<[_; 8]>::with_capacity(to.len());
  let mut added_idx = added.next().map(|k| to.get_full(k).unwrap().0);
  let mut removed_idx = removed.next().map(|k| from.get_full(k).unwrap().0);

  for (idx, k) in to.iter().enumerate() {
    if let Some(added_idx) = added_idx.as_mut().filter(|r_i| **r_i == idx) {
      if let Some(next_added) = added.next().map(|k| to.get_full(k).unwrap().0)
      {
        *added_idx = next_added;

        normalized_idx = usize::wrapping_sub(normalized_idx, 1);
      }
    }

    if let Some(removed_idx) = removed_idx.as_mut().filter(|r_i| **r_i == idx) {
      normalized_idx += 1;

      if let Some(next_removed) =
        removed.next().map(|k| from.get_full(k).unwrap().0)
      {
        *removed_idx = next_removed;
      }
    }

    if let Some((from_idx, _)) = from.get_full(k) {
      if from_idx != normalized_idx {
        move_cmds.push(DiffOpMove {
          from: from_idx,
          to: idx,
          move_in_dom: true,
        });
      } else {
        move_cmds.push(DiffOpMove {
          from: from_idx,
          to: idx,
          move_in_dom: false,
        });
      }
    }

    normalized_idx += 1;
  }

  let mut diffs = Diff {
    removed: removed_cmds.collect(),
    moved: move_cmds,
    added: added_cmds.collect(),
    clear: false,
  };

  apply_opts(from, to, &mut diffs);

  diffs
}

fn apply_opts<K: Eq + Hash>(
  from: &FxIndexSet<K>,
  to: &FxIndexSet<K>,
  cmds: &mut Diff,
) {
  // We can optimize the case of replacing all items
  if !from.is_empty()
    && !to.is_empty()
    && cmds.removed.len() == from.len()
    && cmds.moved.len() == 0
  {
    cmds.clear = true;

    cmds
      .added
      .iter_mut()
      .for_each(|op| op.mode = DiffOpAddMode::Append);

    return;
  }

  // We can optimize appends.
  if cmds.added.len() != 0
    && cmds.moved.len() == 0
    && cmds.removed.len() == 0
    && cmds.added[0].at >= from.len()
  {
    cmds
      .added
      .iter_mut()
      .for_each(|op| op.mode = DiffOpAddMode::Append);
  }
}

#[derive(Debug, Default)]
struct Diff {
  removed: SmallVec<[DiffOpRemove; 8]>,
  moved: SmallVec<[DiffOpMove; 8]>,
  added: SmallVec<[DiffOpAdd; 8]>,
  clear: bool,
}

#[derive(Debug)]
struct DiffOpMove {
  from: usize,
  to: usize,
  move_in_dom: bool,
}

#[derive(Debug)]
struct DiffOpAdd {
  at: usize,
  mode: DiffOpAddMode,
}

#[derive(Debug)]
struct DiffOpRemove {
  at: usize,
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
  if cmds.added.len().checked_sub(cmds.removed.len()).is_some() {
    children.resize_with(
      children.len()
        + (cmds.added.len() as isize - cmds.removed.len() as isize) as usize,
      || None,
    );
  }

  // We need to hold a list of items which will be moved, and
  // we can only perform the omve after all commands have run, otherwise,
  // we risk overwriting one of the values
  let mut items_to_move = Vec::with_capacity(cmds.moved.len());

  debug!("{cmds:#?}");

  // The order of cmds needs to be:
  // 1. Removed
  // 2. Moved
  // 3. Add
  if cmds.clear {
    #[cfg(all(target_arch = "wasm32", feature = "web"))]
    {
      if opening.previous_sibling().is_none()
        && closing.next_sibling().is_none()
      {
        let parent = closing
          .parent_node()
          .unwrap()
          .unchecked_into::<web_sys::Element>();
        parent.set_text_content(Some(""));

        #[cfg(debug_assertions)]
        parent.append_with_node_2(opening, closing).unwrap();

        #[cfg(not(debug_assertions))]
        parent.append_with_node_1(closing).unwrap();
      } else {
        range.set_start_before(opening).unwrap();
        range.set_end_before(closing).unwrap();

        range.delete_contents().unwrap();
      }
    }
  }

  for DiffOpRemove { at } in cmds.removed {
    let item_to_remove = std::mem::take(&mut children[at]).unwrap();

    #[cfg(all(target_arch = "wasm32", feature = "web"))]
    item_to_remove.prepare_for_move();
  }

  for DiffOpMove {
    from,
    to,
    move_in_dom,
  } in cmds.moved
  {
    let item = std::mem::take(&mut children[from]).unwrap();

    #[cfg(all(target_arch = "wasm32", feature = "web"))]
    if move_in_dom {
      item.prepare_for_move()
    }

    items_to_move.push((move_in_dom, to, item));
  }

  for DiffOpAdd { at, mode } in cmds.added {
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

  for (move_in_dom, to, each_item) in items_to_move {
    #[cfg(all(target_arch = "wasm32", feature = "web"))]
    if move_in_dom {
      let opening =
        children.get_next_closest_mounted_sibling(to + 1, closing.to_owned());

      mount_child(MountKind::Before(&opening), &each_item);
    }

    children[to] = Some(each_item);
  }

  // Now, remove the holes that might have been left from removing
  // items
  children.drain_filter(|c| c.is_none());
}
