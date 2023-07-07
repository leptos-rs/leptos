#[cfg(not(all(target_arch = "wasm32", feature = "web")))]
use crate::hydration::HydrationKey;
use crate::{hydration::HydrationCtx, Comment, CoreComponent, IntoView, View};
use leptos_reactive::Scope;
use std::{borrow::Cow, cell::RefCell, fmt, hash::Hash, ops::Deref, rc::Rc};
#[cfg(all(target_arch = "wasm32", feature = "web"))]
use web::*;

#[cfg(all(target_arch = "wasm32", feature = "web"))]
mod web {
    pub(crate) use crate::{
        mount_child, prepare_to_move, MountKind, Mountable, RANGE,
    };
    pub use drain_filter_polyfill::VecExt as VecDrainFilterExt;
    pub use leptos_reactive::create_effect;
    pub use once_cell::unsync::OnceCell;
    pub use wasm_bindgen::JsCast;
}

#[allow(dead_code)] // not used in SSR
type FxIndexSet<T> =
    indexmap::IndexSet<T, std::hash::BuildHasherDefault<rustc_hash::FxHasher>>;

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
#[derive(Clone, PartialEq, Eq)]
pub struct EachRepr {
    #[cfg(all(target_arch = "wasm32", feature = "web"))]
    document_fragment: web_sys::DocumentFragment,
    #[cfg(all(target_arch = "wasm32", feature = "web"))]
    mounted: Rc<OnceCell<()>>,
    #[cfg(debug_assertions)]
    opening: Comment,
    pub(crate) children: Rc<RefCell<Vec<Option<EachItem>>>>,
    closing: Comment,
    #[cfg(not(all(target_arch = "wasm32", feature = "web")))]
    pub(crate) id: HydrationKey,
}

impl fmt::Debug for EachRepr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use fmt::Write;

        f.write_str("<Each>\n")?;

        for child in self.children.borrow().deref() {
            let mut pad_adapter = pad_adapter::PadAdapter::new(f);

            writeln!(pad_adapter, "{:#?}", child.as_ref().unwrap())?;
        }

        f.write_str("</Each>")
    }
}

impl Default for EachRepr {
    fn default() -> Self {
        let id = HydrationCtx::id();

        let markers = (
            Comment::new(Cow::Borrowed("</Each>"), &id, true),
            #[cfg(debug_assertions)]
            Comment::new(Cow::Borrowed("<Each>"), &id, false),
        );

        #[cfg(all(target_arch = "wasm32", feature = "web"))]
        let document_fragment = {
            let fragment = crate::document().create_document_fragment();

            // Insert the comments into the document fragment
            #[cfg(debug_assertions)]
            // so they can serve as our references when inserting
            // future nodes
            if !HydrationCtx::is_hydrating() {
                fragment
                    .append_with_node_2(&markers.1.node, &markers.0.node)
                    .expect("append to not err");
            }

            #[cfg(not(debug_assertions))]
            if !HydrationCtx::is_hydrating() {
                fragment.append_with_node_1(&markers.0.node).unwrap();
            }

            fragment
        };

        Self {
            #[cfg(all(target_arch = "wasm32", feature = "web"))]
            document_fragment,
            #[cfg(all(target_arch = "wasm32", feature = "web"))]
            mounted: Default::default(),
            #[cfg(debug_assertions)]
            opening: markers.1,
            children: Default::default(),
            closing: markers.0,
            #[cfg(not(all(target_arch = "wasm32", feature = "web")))]
            id,
        }
    }
}

#[cfg(all(target_arch = "wasm32", feature = "web"))]
impl Mountable for EachRepr {
    fn get_mountable_node(&self) -> web_sys::Node {
        if self.mounted.get().is_none() {
            self.mounted.set(()).unwrap();

            self.document_fragment.clone().unchecked_into()
        } else {
            let opening = self.get_opening_node();

            prepare_to_move(
                &self.document_fragment,
                &opening,
                &self.closing.node,
            );

            self.document_fragment.clone().unchecked_into()
        }
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

    #[inline(always)]
    fn get_closing_node(&self) -> web_sys::Node {
        self.closing.node.clone()
    }
}

/// The internal representation of an [`Each`] item.
#[derive(PartialEq, Eq)]
pub(crate) struct EachItem {
    cx: Scope,
    #[cfg(all(target_arch = "wasm32", feature = "web"))]
    document_fragment: Option<web_sys::DocumentFragment>,
    #[cfg(debug_assertions)]
    opening: Option<Comment>,
    pub(crate) child: View,
    closing: Option<Comment>,
    #[cfg(not(all(target_arch = "wasm32", feature = "web")))]
    pub(crate) id: HydrationKey,
}

impl fmt::Debug for EachItem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use fmt::Write;

        f.write_str("<EachItem>\n")?;

        let mut pad_adapter = pad_adapter::PadAdapter::new(f);

        writeln!(pad_adapter, "{:#?}", self.child)?;

        f.write_str("</EachItem>")
    }
}

impl EachItem {
    fn new(cx: Scope, child: View) -> Self {
        let id = HydrationCtx::id();
        let needs_closing = !matches!(child, View::Element(_));

        let markers = (
            if needs_closing {
                Some(Comment::new(Cow::Borrowed("</EachItem>"), &id, true))
            } else {
                None
            },
            #[cfg(debug_assertions)]
            if needs_closing {
                Some(Comment::new(Cow::Borrowed("<EachItem>"), &id, false))
            } else {
                None
            },
        );

        #[cfg(all(target_arch = "wasm32", feature = "web"))]
        let document_fragment = if needs_closing {
            let fragment = crate::document().create_document_fragment();
            let closing = markers.0.as_ref().unwrap();

            // Insert the comments into the document fragment
            // so they can serve as our references when inserting
            // future nodes
            if !HydrationCtx::is_hydrating() {
                #[cfg(debug_assertions)]
                fragment
                    .append_with_node_2(
                        &markers.1.as_ref().unwrap().node,
                        &closing.node,
                    )
                    .unwrap();
                fragment.append_with_node_1(&closing.node).unwrap();
            }

            // if child view is Text and if we are hydrating, we do not
            // need to mount it. otherwise, mount it here
            if !HydrationCtx::is_hydrating() || !matches!(child, View::Text(_))
            {
                mount_child(MountKind::Before(&closing.node), &child);
            }

            Some(fragment)
        } else {
            None
        };

        Self {
            cx,
            #[cfg(all(target_arch = "wasm32", feature = "web"))]
            document_fragment,
            #[cfg(debug_assertions)]
            opening: markers.1,
            child,
            closing: markers.0,
            #[cfg(not(all(target_arch = "wasm32", feature = "web")))]
            id,
        }
    }
}

#[cfg(all(target_arch = "wasm32", feature = "web"))]
impl Drop for EachItem {
    fn drop(&mut self) {
        self.cx.dispose();
    }
}

#[cfg(all(target_arch = "wasm32", feature = "web"))]
impl Mountable for EachItem {
    fn get_mountable_node(&self) -> web_sys::Node {
        if let Some(fragment) = &self.document_fragment {
            fragment.clone().unchecked_into()
        } else {
            self.child.get_mountable_node()
        }
    }

    #[inline(always)]
    fn get_opening_node(&self) -> web_sys::Node {
        return self.child.get_opening_node();
    }

    fn get_closing_node(&self) -> web_sys::Node {
        if let Some(closing) = &self.closing {
            closing.node.clone().unchecked_into()
        } else {
            self.child.get_mountable_node().clone()
        }
    }
}

impl EachItem {
    /// Moves all child nodes into its' `DocumentFragment` in
    /// order to be reinserted somewhere else.
    #[cfg(all(target_arch = "wasm32", feature = "web"))]
    fn prepare_for_move(&self) {
        if let Some(fragment) = &self.document_fragment {
            let start = self.get_opening_node();
            let end = &self.get_closing_node();

            let mut sibling = start;

            while sibling != *end {
                let next_sibling = sibling.next_sibling().unwrap();

                fragment.append_child(&sibling).unwrap();

                sibling = next_sibling;
            }

            fragment.append_with_node_1(end).unwrap();
        } else {
            let node = self.child.get_mountable_node();
            node.unchecked_into::<web_sys::Element>().remove();
        }
    }
}

/// A component for efficiently rendering an iterable.
pub struct Each<IF, I, T, EF, N, KF, K>
where
    IF: Fn() -> I + 'static,
    I: IntoIterator<Item = T>,
    EF: Fn(Scope, T) -> N + 'static,
    N: IntoView,
    KF: Fn(&T) -> K + 'static,
    K: Eq + Hash + 'static,
    T: 'static,
{
    pub(crate) items_fn: IF,
    pub(crate) each_fn: EF,
    key_fn: KF,
}

impl<IF, I, T, EF, N, KF, K> Each<IF, I, T, EF, N, KF, K>
where
    IF: Fn() -> I + 'static,
    I: IntoIterator<Item = T>,
    EF: Fn(Scope, T) -> N + 'static,
    N: IntoView,
    KF: Fn(&T) -> K,
    K: Eq + Hash + 'static,
    T: 'static,
{
    /// Creates a new [`Each`] component.
    #[inline(always)]
    pub const fn new(items_fn: IF, key_fn: KF, each_fn: EF) -> Self {
        Self {
            items_fn,
            each_fn,
            key_fn,
        }
    }
}

impl<IF, I, T, EF, N, KF, K> IntoView for Each<IF, I, T, EF, N, KF, K>
where
    IF: Fn() -> I + 'static,
    I: IntoIterator<Item = T>,
    EF: Fn(Scope, T) -> N + 'static,
    N: IntoView,
    KF: Fn(&T) -> K + 'static,
    K: Eq + Hash + 'static,
    T: 'static,
{
    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(level = "info", name = "<Each />", skip_all)
    )]
    fn into_view(self, cx: Scope) -> crate::View {
        let Self {
            items_fn,
            each_fn,
            key_fn,
        } = self;

        #[cfg(not(all(target_arch = "wasm32", feature = "web")))]
        let _ = key_fn;

        let component = EachRepr::default();

        #[cfg(all(target_arch = "wasm32", feature = "web"))]
        let (children, closing) =
            (component.children.clone(), component.closing.node.clone());

        #[cfg(all(target_arch = "wasm32", feature = "web"))]
        create_effect(
            cx,
            move |prev_hash_run: Option<HashRun<FxIndexSet<K>>>| {
                let mut children_borrow = children.borrow_mut();

                #[cfg(all(target_arch = "wasm32", feature = "web"))]
                let opening = if let Some(Some(child)) = children_borrow.get(0)
                {
                    // correctly remove opening <!--<EachItem/>-->
                    let child_opening = child.get_opening_node();
                    #[cfg(debug_assertions)]
                    {
                        use crate::components::dyn_child::NonViewMarkerSibling;
                        child_opening
                            .previous_non_view_marker_sibling()
                            .unwrap_or(child_opening)
                    }
                    #[cfg(not(debug_assertions))]
                    {
                        child_opening
                    }
                } else {
                    closing.clone()
                };

                let items_iter = items_fn().into_iter();

                let (capacity, _) = items_iter.size_hint();
                let mut hashed_items = FxIndexSet::with_capacity_and_hasher(
                    capacity,
                    Default::default(),
                );

                if let Some(HashRun(prev_hash_run)) = prev_hash_run {
                    if !prev_hash_run.is_empty() {
                        let mut items = Vec::with_capacity(capacity);
                        for item in items_iter {
                            hashed_items.insert(key_fn(&item));
                            items.push(Some(item));
                        }

                        let cmds = diff(&prev_hash_run, &hashed_items);

                        apply_diff(
                            cx,
                            #[cfg(all(
                                target_arch = "wasm32",
                                feature = "web"
                            ))]
                            &opening,
                            #[cfg(all(
                                target_arch = "wasm32",
                                feature = "web"
                            ))]
                            &closing,
                            cmds,
                            &mut children_borrow,
                            items,
                            &each_fn,
                        );
                        return HashRun(hashed_items);
                    }
                }

                // if previous run is empty
                *children_borrow = Vec::with_capacity(capacity);
                #[cfg(all(target_arch = "wasm32", feature = "web"))]
                let fragment = crate::document().create_document_fragment();

                for item in items_iter {
                    hashed_items.insert(key_fn(&item));
                    let (each_item, _) = cx.run_child_scope(|cx| {
                        EachItem::new(cx, each_fn(cx, item).into_view(cx))
                    });
                    #[cfg(all(target_arch = "wasm32", feature = "web"))]
                    {
                        _ = fragment
                            .append_child(&each_item.get_mountable_node());
                    }

                    children_borrow.push(Some(each_item));
                }

                #[cfg(all(target_arch = "wasm32", feature = "web"))]
                closing
                    .unchecked_ref::<web_sys::Element>()
                    .before_with_node_1(&fragment)
                    .expect("before to not err");

                HashRun(hashed_items)
            },
        );

        #[cfg(not(all(target_arch = "wasm32", feature = "web")))]
        {
            *component.children.borrow_mut() = (items_fn)()
                .into_iter()
                .map(|child| {
                    cx.run_child_scope(|cx| {
                        Some(EachItem::new(
                            cx,
                            (each_fn)(cx, child).into_view(cx),
                        ))
                    })
                    .0
                })
                .collect();
        }

        View::CoreComponent(CoreComponent::Each(component))
    }
}

#[derive(educe::Educe)]
#[educe(Debug)]
struct HashRun<T>(#[educe(Debug(ignore))] T);

/// Calculates the operations needed to get from `from` to `to`.
#[allow(dead_code)] // not used in SSR but useful to have available for testing
fn diff<K: Eq + Hash>(from: &FxIndexSet<K>, to: &FxIndexSet<K>) -> Diff {
    if from.is_empty() && to.is_empty() {
        return Diff::default();
    } else if to.is_empty() {
        return Diff {
            clear: true,
            ..Default::default()
        };
    } else if from.is_empty() {
        return Diff {
            added: to
                .iter()
                .enumerate()
                .map(|(at, _)| DiffOpAdd {
                    at,
                    mode: DiffOpAddMode::Append,
                })
                .collect(),
            ..Default::default()
        };
    }

    let mut removed = vec![];
    let mut moved = vec![];
    let mut added = vec![];
    let max_len = std::cmp::max(from.len(), to.len());

    for index in 0..max_len {
        let from_item = from.get_index(index);
        let to_item = to.get_index(index);

        // if they're the same, do nothing
        if from_item != to_item {
            // if it's only in old, not new, remove it
            if from_item.is_some() && !to.contains(from_item.unwrap()) {
                let op = DiffOpRemove { at: index };
                removed.push(op);
            }
            // if it's only in new, not old, add it
            if to_item.is_some() && !from.contains(to_item.unwrap()) {
                let op = DiffOpAdd {
                    at: index,
                    mode: DiffOpAddMode::Normal,
                };
                added.push(op);
            }
            // if it's in both old and new, it can either
            // 1) be moved (and need to move in the DOM)
            // 2) be moved (but not need to move in the DOM)
            //    * this would happen if, for example, 2 items
            //      have been added before it, and it has moved by 2
            if let Some(from_item) = from_item {
                if let Some(to_item) = to.get_full(from_item) {
                    let moves_forward_by = (to_item.0 as i32) - (index as i32);
                    let move_in_dom = moves_forward_by
                        != (added.len() as i32) - (removed.len() as i32);

                    let op = DiffOpMove {
                        from: index,
                        len: 1,
                        to: to_item.0,
                        move_in_dom,
                    };
                    moved.push(op);
                }
            }
        }
    }

    moved = group_adjacent_moves(moved);

    Diff {
        removed,
        items_to_move: moved.iter().map(|m| m.len).sum(),
        moved,
        added,
        clear: false,
    }
}

/// Group adjacent items that are being moved as a group.
/// For example from `[2, 3, 5, 6]` to `[1, 2, 3, 4, 5, 6]` should result
/// in a move for `2,3` and `5,6` rather than 4 individual moves.
fn group_adjacent_moves(moved: Vec<DiffOpMove>) -> Vec<DiffOpMove> {
    let mut prev: Option<DiffOpMove> = None;
    let mut new_moved = Vec::with_capacity(moved.len());
    for m in moved {
        match prev {
            Some(mut p) => {
                if (m.from == p.from + p.len) && (m.to == p.to + p.len) {
                    p.len += 1;
                    prev = Some(p);
                } else {
                    new_moved.push(prev.take().unwrap());
                    prev = Some(m);
                }
            }
            None => prev = Some(m),
        }
    }
    if let Some(prev) = prev {
        new_moved.push(prev)
    }
    new_moved
}

#[derive(Debug, Default, PartialEq, Eq)]
struct Diff {
    removed: Vec<DiffOpRemove>,
    moved: Vec<DiffOpMove>,
    items_to_move: usize,
    added: Vec<DiffOpAdd>,
    clear: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct DiffOpMove {
    /// The index this range is starting relative to `from`.
    from: usize,
    /// The number of elements included in this range.
    len: usize,
    /// The starting index this range will be moved to relative to `to`.
    to: usize,
    /// Marks this move to be applied to the DOM, or just to the underlying
    /// storage
    move_in_dom: bool,
}

impl Default for DiffOpMove {
    fn default() -> Self {
        Self {
            from: 0,
            to: 0,
            len: 1,
            move_in_dom: true,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct DiffOpAdd {
    at: usize,
    mode: DiffOpAddMode,
}

#[derive(Debug, PartialEq, Eq)]
struct DiffOpRemove {
    at: usize,
}

#[allow(dead_code)] // Append not used in SSR but useful
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DiffOpAddMode {
    Normal,
    Append,
    // Todo
    _Prepend,
}

impl Default for DiffOpAddMode {
    fn default() -> Self {
        Self::Normal
    }
}

#[cfg(all(target_arch = "wasm32", feature = "web"))]
fn apply_diff<T, EF, V>(
    cx: Scope,
    opening: &web_sys::Node,
    closing: &web_sys::Node,
    diff: Diff,
    children: &mut Vec<Option<EachItem>>,
    mut items: Vec<Option<T>>,
    each_fn: &EF,
) where
    EF: Fn(Scope, T) -> V,
    V: IntoView,
{
    let range = RANGE.with(|range| (*range).clone());

    // The order of cmds needs to be:
    // 1. Clear
    // 2. Removals
    // 3. Move out
    // 4. Resize
    // 5. Move in
    // 6. Additions
    // 7. Removes holes
    if diff.clear {
        if opening.previous_sibling().is_none()
            && closing.next_sibling().is_none()
        {
            let parent = closing
                .parent_node()
                .expect("could not get closing node")
                .unchecked_into::<web_sys::Element>();
            parent.set_text_content(Some(""));

            #[cfg(debug_assertions)]
            parent.append_with_node_2(opening, closing).unwrap();

            #[cfg(not(debug_assertions))]
            parent.append_with_node_1(closing).unwrap();
        } else {
            #[cfg(debug_assertions)]
            range.set_start_after(opening).unwrap();
            #[cfg(not(debug_assertions))]
            range.set_start_before(opening).unwrap();

            range.set_end_before(closing).unwrap();

            range.delete_contents().unwrap();
        }

        children.clear();

        if diff.added.is_empty() {
            return;
        }
    }

    for DiffOpRemove { at } in &diff.removed {
        let item_to_remove = children[*at].take().unwrap();

        item_to_remove.prepare_for_move();
    }

    let (move_cmds, add_cmds) = unpack_moves(&diff);

    let mut moved_children = move_cmds
        .iter()
        .map(|move_| {
            let each_item = children[move_.from].take().unwrap();

            if move_.move_in_dom {
                each_item.prepare_for_move();
            }

            Some(each_item)
        })
        .collect::<Vec<_>>();

    children.resize_with(children.len() + diff.added.len(), || None);

    for (i, DiffOpMove { to, .. }) in move_cmds
        .iter()
        .enumerate()
        .filter(|(_, move_)| !move_.move_in_dom)
    {
        children[*to] = moved_children[i].take();
    }

    for (i, DiffOpMove { to, .. }) in move_cmds
        .into_iter()
        .enumerate()
        .filter(|(_, move_)| move_.move_in_dom)
    {
        let each_item = moved_children[i].take().unwrap();

        let sibling_node =
            children.get_next_closest_mounted_sibling(to, closing.to_owned());

        mount_child(MountKind::Before(&sibling_node), &each_item);

        children[to] = Some(each_item);
    }

    for DiffOpAdd { at, mode } in add_cmds {
        let (each_item, _) = cx.run_child_scope(|cx| {
            let view = each_fn(cx, items[at].take().unwrap()).into_view(cx);

            EachItem::new(cx, view)
        });

        match mode {
            DiffOpAddMode::Normal => {
                let sibling_node = children
                    .get_next_closest_mounted_sibling(at, closing.to_owned());

                mount_child(MountKind::Before(&sibling_node), &each_item);
            }
            DiffOpAddMode::Append => {
                mount_child(MountKind::Before(closing), &each_item);
            }
            DiffOpAddMode::_Prepend => {
                todo!("Prepends are not yet implemented")
            }
        }

        children[at] = Some(each_item);
    }

    #[allow(unstable_name_collisions)]
    children.drain_filter(|c| c.is_none());
}

/// Unpacks adds and moves into a sequence of interleaved
/// add and move commands. Move commands will always return
/// with a `len == 1`.
#[cfg(all(target_arch = "wasm32", feature = "web"))]
fn unpack_moves(diff: &Diff) -> (Vec<DiffOpMove>, Vec<DiffOpAdd>) {
    let mut moves = Vec::with_capacity(diff.items_to_move);
    let mut adds = Vec::with_capacity(diff.added.len());

    let mut removes_iter = diff.removed.iter();
    let mut adds_iter = diff.added.iter();
    let mut moves_iter = diff.moved.iter();

    let mut removes_next = removes_iter.next();
    let mut adds_next = adds_iter.next();
    let mut moves_next = moves_iter.next().copied();

    for i in 0..diff.items_to_move + diff.added.len() + diff.removed.len() {
        if let Some(DiffOpRemove { at, .. }) = removes_next {
            if i == *at {
                removes_next = removes_iter.next();

                continue;
            }
        }

        match (adds_next, &mut moves_next) {
            (Some(add), Some(move_)) => {
                if add.at == i {
                    adds.push(*add);

                    adds_next = adds_iter.next();
                } else {
                    let mut single_move = *move_;
                    single_move.len = 1;

                    moves.push(single_move);

                    move_.len -= 1;
                    move_.from += 1;
                    move_.to += 1;

                    if move_.len == 0 {
                        moves_next = moves_iter.next().copied();
                    }
                }
            }
            (Some(add), None) => {
                adds.push(*add);

                adds_next = adds_iter.next();
            }
            (None, Some(move_)) => {
                let mut single_move = *move_;
                single_move.len = 1;

                moves.push(single_move);

                move_.len -= 1;
                move_.from += 1;
                move_.to += 1;

                if move_.len == 0 {
                    moves_next = moves_iter.next().copied();
                }
            }
            (None, None) => break,
        }
    }

    (moves, adds)
}

#[cfg(test)]
mod test_utils {
    use super::*;

    pub trait IntoFxIndexSet<K> {
        fn into_fx_index_set(self) -> FxIndexSet<K>;
    }

    impl<T, K> IntoFxIndexSet<K> for T
    where
        T: IntoIterator<Item = K>,
        K: Eq + Hash,
    {
        fn into_fx_index_set(self) -> FxIndexSet<K> {
            self.into_iter().collect()
        }
    }
}

#[cfg(test)]
use test_utils::*;

#[cfg(test)]
mod diff {
    use super::*;

    #[test]
    fn only_adds() {
        let diff =
            diff(&[].into_fx_index_set(), &[1, 2, 3].into_fx_index_set());

        assert_eq!(
            diff,
            Diff {
                added: vec![
                    DiffOpAdd {
                        at: 0,
                        mode: DiffOpAddMode::Append
                    },
                    DiffOpAdd {
                        at: 1,
                        mode: DiffOpAddMode::Append
                    },
                    DiffOpAdd {
                        at: 2,
                        mode: DiffOpAddMode::Append
                    },
                ],
                ..Default::default()
            }
        );
    }

    #[test]
    fn only_removes() {
        let diff =
            diff(&[1, 2, 3].into_fx_index_set(), &[3].into_fx_index_set());

        assert_eq!(
            diff,
            Diff {
                removed: vec![DiffOpRemove { at: 0 }, DiffOpRemove { at: 1 }],
                moved: vec![DiffOpMove {
                    from: 2,
                    len: 1,
                    to: 0,
                    move_in_dom: false
                }],
                items_to_move: 1,
                ..Default::default()
            }
        );
    }

    #[test]
    fn adds_with_no_move() {
        let diff =
            diff(&[3].into_fx_index_set(), &[1, 2, 3].into_fx_index_set());

        assert_eq!(
            diff,
            Diff {
                added: vec![
                    DiffOpAdd {
                        at: 0,
                        ..Default::default()
                    },
                    DiffOpAdd {
                        at: 1,
                        ..Default::default()
                    },
                ],
                moved: vec![DiffOpMove {
                    from: 0,
                    len: 1,
                    to: 2,
                    move_in_dom: true
                }],
                items_to_move: 1,
                ..Default::default()
            }
        );
    }

    #[test]
    fn move_as_group() {
        let diff = diff(
            &[2, 3, 4, 5].into_fx_index_set(),
            &[1, 2, 3, 4, 5].into_fx_index_set(),
        );

        assert_eq!(
            diff,
            Diff {
                added: vec![DiffOpAdd {
                    at: 0,
                    ..Default::default()
                },],
                moved: vec![DiffOpMove {
                    from: 0,
                    len: 4,
                    to: 1,
                    move_in_dom: false
                },],
                items_to_move: 4,
                ..Default::default()
            }
        );
    }

    #[test]
    fn move_as_group_with_gap() {
        let diff = diff(
            &[2, 3, 5, 6].into_fx_index_set(),
            &[1, 2, 3, 4, 5, 6].into_fx_index_set(),
        );

        assert_eq!(
            diff,
            Diff {
                added: vec![
                    DiffOpAdd {
                        at: 0,
                        ..Default::default()
                    },
                    DiffOpAdd {
                        at: 3,
                        ..Default::default()
                    },
                ],
                moved: vec![
                    DiffOpMove {
                        from: 0,
                        len: 2,
                        to: 1,
                        move_in_dom: false
                    },
                    DiffOpMove {
                        from: 2,
                        len: 2,
                        to: 4,
                        move_in_dom: true
                    }
                ],
                items_to_move: 4,
                ..Default::default()
            }
        );
    }
}
