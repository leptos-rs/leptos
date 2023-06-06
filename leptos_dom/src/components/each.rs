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
    pub use std::cell::OnceCell;
    pub use wasm_bindgen::JsCast;
}

#[cfg(any(test, all(target_arch = "wasm32", feature = "web")))]
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

            mount_child(MountKind::Before(&closing.node), &child);

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
                    child.get_opening_node()
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

                        apply_cmds(
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

/// Calculates the operations need to get from `a` to `b`.
#[cfg(any(test, all(target_arch = "wasm32", feature = "web")))]
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

    // Get removed items
    let mut removed = from.difference(to);

    let remove_cmds = removed
        .clone()
        .map(|k| from.get_full(k).unwrap().0)
        .map(|idx| DiffOpRemove { at: idx });

    // Get added items
    let mut added = to.difference(from);

    let add_cmds =
        added
            .clone()
            .map(|k| to.get_full(k).unwrap().0)
            .map(|idx| DiffOpAdd {
                at: idx,
                mode: Default::default(),
            });

    // Get items that might have moved
    let from_moved = from.intersection(&to).collect::<FxIndexSet<_>>();
    let to_moved = to.intersection(&from).collect::<FxIndexSet<_>>();

    let move_cmds = find_ranges(from_moved, to_moved, from, to);

    let mut diffs = Diff {
        removed: remove_cmds.collect(),
        moved: move_cmds,
        added: add_cmds.collect(),
        clear: false,
    };

    apply_opts(from, to, &mut diffs);

    diffs
}

#[cfg(any(test, all(target_arch = "wasm32", feature = "web")))]
fn find_ranges<K: Eq + Hash>(
    from_moved: FxIndexSet<&K>,
    to_moved: FxIndexSet<&K>,
    from: &FxIndexSet<K>,
    to: &FxIndexSet<K>,
) -> Vec<DiffOpMove> {
    // let mut ranges = vec![];

    use drain_filter_polyfill::VecExt;

    let mut ranges = Vec::with_capacity(from.len());
    let mut prev_to_moved_index = 0;
    let mut range = DiffOpMove::default();

    for (i, k) in from_moved.into_iter().enumerate() {
        let to_moved_index = to_moved.get_index_of(k).unwrap();

        if i == 0 {
            range.from = from.get_index_of(k).unwrap();
            range.to = to.get_index_of(k).unwrap();
        }
        // The range continues
        else if to_moved_index == prev_to_moved_index + 1 {
            range.len += 1;

            // Are we still dense?
            if !(range.is_dense
                && to.get_index_of(k).unwrap() == range.to + range.len - 1)
            {
                range.is_dense = false;
            }
        }
        // We're done with this range, start a new one
        else {
            ranges.push(std::mem::take(&mut range));

            range.from = from.get_index_of(k).unwrap();
            range.to = to.get_index_of(k).unwrap();
        }

        prev_to_moved_index = to_moved_index;

        &range;
    }

    ranges.push(std::mem::take(&mut range));

    // We need to remove ranges that didn't move relative to each other
    let mut to_ranges = ranges.clone();
    to_ranges.sort_unstable_by_key(|range| range.to);

    let mut filtered_ranges = vec![];

    for (i, range) in ranges.into_iter().enumerate() {
        if range != to_ranges[i] {
            filtered_ranges.push(range);
        }
    }

    filtered_ranges
}

#[cfg(any(test, all(target_arch = "wasm32", feature = "web")))]
fn apply_opts<K: Eq + Hash>(
    from: &FxIndexSet<K>,
    to: &FxIndexSet<K>,
    cmds: &mut Diff,
) {
    // We can optimize the case of replacing all items
    if !from.is_empty()
        && !to.is_empty()
        && cmds.removed.len() == from.len()
        && cmds.moved.is_empty()
    {
        cmds.clear = true;

        cmds.added
            .iter_mut()
            .for_each(|op| op.mode = DiffOpAddMode::Append);

        return;
    }

    // We can optimize appends.
    if !cmds.added.is_empty()
        && cmds.moved.is_empty()
        && cmds.removed.is_empty()
        && cmds.added[0].at >= from.len()
    {
        cmds.added
            .iter_mut()
            .for_each(|op| op.mode = DiffOpAddMode::Append);
    }

    optimize_moves(&mut cmds.moved);
}

#[cfg(any(test, all(target_arch = "wasm32", feature = "web")))]
fn optimize_moves(moves: &mut Vec<DiffOpMove>) {
    // This is the easiest optimal move case, which is to
    // simply swap the 2 ranges. We only need to move the range
    // that is smallest.
    if moves.len() == 2 {
        if moves[1].len < moves[0].len {
            moves.remove(0);
        } else {
            moves.pop();
        }
    }
    // Interestingly enough, there are NO configuration that are possible
    // for ranges of 3.
    //
    // For example, take A, B, C. Here are all possible configurations and
    // reasons for why they are impossible:
    // - A B C  # identity, would be removed by ranges that didn't move
    // - A C B  # `A` would be removed, thus it's a case of length 2
    // - B A C  # `C` would be removed, thus it's a case of length 2
    // - B C A  # `B C` are congiguous, so this is would have been a single range
    // - C A B  # `A B` are congiguous, so this is would have been a single range
    // - C B A  # `B` would be removed, thus it's a case of length 2
    //
    // We can add more pre-computed tables here if benchmarking or
    // user demand needs it...nevertheless, it is unlikely for us
    // to implement this algorithm to handle N ranges, because this
    // becomes exponentially more expensive to compute. It's faster,
    // for the most part, to assume the ranges are random and move
    // all the ranges around than to try and figure out the best way
    // to move them
    else {
        // The idea here is that for N ranges, we never need to
        // move the largest range, rather, have all ranges move
        // around it. It might be deemed faster to remove this
        // if benchmarking shows to be too slow for large N, because
        // this statement here makes worst case O(n * log(n)) because
        // of sorting, whereas without it, it's O(n).
        //
        // Although O(n * log(n)) sounds worse than O(n), for small
        // n, this is going to be faster, because updating the DOM
        // is expensive. We should benchmark to find the crossover point,
        // and add this condition here
        moves.sort_unstable_by_key(|range| range.len);
        moves.pop();
    }
}

#[cfg(any(test, all(target_arch = "wasm32", feature = "web")))]
#[derive(Debug, Default)]
struct Diff {
    removed: Vec<DiffOpRemove>,
    moved: Vec<DiffOpMove>,
    added: Vec<DiffOpAdd>,
    clear: bool,
}

#[cfg(any(test, all(target_arch = "wasm32", feature = "web")))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct DiffOpMove {
    /// The index this range is starting relative to `from`.
    from: usize,
    /// The number of elements included in this range.
    len: usize,
    /// The starting index this range will be moved to relative to `to`.
    to: usize,
    /// Set to true when the range is fully contiguous, i.e., does not
    /// have any inserted items in the middle of the range with respect
    /// to `to`.
    is_dense: bool,
}

#[cfg(any(test, all(target_arch = "wasm32", feature = "web")))]
impl Default for DiffOpMove {
    fn default() -> Self {
        Self {
            from: 0,
            to: 0,
            len: 1,
            is_dense: true,
        }
    }
}

#[cfg(any(test, all(target_arch = "wasm32", feature = "web")))]
#[derive(Debug, Default)]
struct DiffOpAdd {
    at: usize,
    mode: DiffOpAddMode,
}

#[cfg(any(test, all(target_arch = "wasm32", feature = "web")))]
#[derive(Debug)]
struct DiffOpRemove {
    at: usize,
}

#[cfg(any(test, all(target_arch = "wasm32", feature = "web")))]
#[derive(Debug)]
enum DiffOpAddMode {
    Normal,
    Append,
    // Todo
    _Prepend,
}

#[cfg(any(test, all(target_arch = "wasm32", feature = "web")))]
impl Default for DiffOpAddMode {
    fn default() -> Self {
        Self::Normal
    }
}

#[cfg(all(target_arch = "wasm32", feature = "web"))]
fn apply_cmds<T, EF, V>(
    cx: Scope,
    opening: &web_sys::Node,
    closing: &web_sys::Node,
    mut cmds: Diff,
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
    // 3. Remove holes left from removals
    // 4. Moves + Add
    if cmds.clear {
        cmds.removed.clear();

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
            range.set_start_before(opening).unwrap();
            range.set_end_before(closing).unwrap();

            range.delete_contents().unwrap();
        }
    }

    for DiffOpRemove { at } in &cmds.removed {
        let item_to_remove = std::mem::take(&mut children[*at]).unwrap();

        item_to_remove.prepare_for_move();
    }

    // Now, remove the holes that might have been left from removing
    // items
    #[allow(unstable_name_collisions)]
    children.drain_filter(|c| c.is_none());

    // Resize children if needed
    if let Some(added) = cmds.added.len().checked_sub(cmds.removed.len()) {
        let target_size = children.len() + added;

        children.resize_with(target_size, || None);
    }

    // Since ranges can have adds in the middle of them, we need to make
    // sure moves and adds happen at the same time in the same order,
    // otherwise, items won't go to the right places
    let mut moves = cmds.moved;
    // TODO: Future optimization, we can get rid of this sort
    // if we change `moves` to be collected from `to` rather than
    // `from`...but I'm too lazy to do it now. If not, let Tom do it,
    // he's a genius.
    moves.sort_unstable_by_key(|range| range.to);

    // We first need to move moves out of children, then we can place
    // them where they need to go, otherwise, we risk overwriting them
    let mut moved_children = Vec::with_capacity(moves.len());

    moves.iter().for_each(|range| {
        for i in range.from..range.from + range.len {
            let child = children[i].take().unwrap();
            child.prepare_for_move();

            moved_children.push(Some(child));
        }
    });

    let mut added_iter = cmds.added.into_iter();
    let mut moves_iter = moves.into_iter();

    let mut added_next = added_iter.next();
    let mut move_next = moves_iter.next();

    loop {
        let mut add_item =
            |add: DiffOpAdd, children: &mut Vec<Option<EachItem>>| {
                let item = items[add.at].take();

                let child = each_fn(cx, item.unwrap()).into_view(cx);

                let each_item = EachItem::new(cx, child);

                match add.mode {
                    DiffOpAddMode::Normal => {
                        let sibling_node = children
                            .get_next_closest_mounted_sibling(
                                add.at,
                                closing.to_owned(),
                            );

                        mount_child(
                            MountKind::Before(&sibling_node),
                            &each_item,
                        );
                    }
                    DiffOpAddMode::Append => {
                        mount_child(MountKind::Before(closing), &each_item);
                    }
                    DiffOpAddMode::_Prepend => todo!(),
                }

                children[add.at] = Some(each_item);
            };

        match (added_iter.next(), moves_iter.next()) {
            (Some(add), Some(move_)) => {
                let mut add = add;
                let mut move_ = move_;

                // Add items that need to be added before the first range,
                // if any
                if add.at < move_.to {
                    add = added_iter.next().unwrap_or_default();

                    added_next = added_iter.next();
                } else {
                    if move_.is_dense {
                        for i in move_.from..move_.len {
                            let child = moved_children[i].take().unwrap();

                            let sibling_node = children
                                .get_next_closest_mounted_sibling(
                                    move_.to,
                                    closing.to_owned(),
                                );

                            mount_child(
                                MountKind::Before(&sibling_node),
                                &child,
                            );

                            children[i] = Some(child);
                        }

                        move_next = moves_iter.next();
                    } else {
                        let each_item =
                            moved_children[move_.from].take().unwrap();

                        let sibling_node = children
                            .get_next_closest_mounted_sibling(
                                move_.to,
                                closing.to_owned(),
                            );

                        mount_child(
                            MountKind::Before(&sibling_node),
                            &each_item,
                        );

                        children[move_.to] = Some(each_item);

                        move_.from += 1;
                        move_.to += 1;
                        move_.len -= 1;

                        if move_.len == 0 {
                            move_next = moves_iter.next();
                        }
                    }
                }
            }
            (Some(add), None) => {
                add_item(add, children);

                added_next = added_iter.next();
            }
            (None, Some(move_)) => {
                let each_item = moved_children[move_.from].take();

                children[move_.to] = each_item;

                move_next = moves_iter.next();
            }
            (None, None) => break,
        }
    }
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
mod find_ranges_tests {
    use super::*;

    // Single range tests will be empty because of removing ranges
    // that didn't move
    #[test]
    fn single_range() {
        let ranges = find_ranges(
            [1, 2, 3, 4].iter().into_fx_index_set(),
            [1, 2, 3, 4].iter().into_fx_index_set(),
            &[1, 2, 3, 4].into_fx_index_set(),
            &[1, 2, 3, 4].into_fx_index_set(),
        );

        assert_eq!(ranges, vec![]);
    }

    #[test]
    fn single_range_with_adds() {
        let ranges = find_ranges(
            [1, 2, 3, 4].iter().into_fx_index_set(),
            [1, 2, 3, 4].iter().into_fx_index_set(),
            &[1, 2, 3, 4].into_fx_index_set(),
            &[1, 2, 5, 3, 4].into_fx_index_set(),
        );

        assert_eq!(ranges, vec![]);
    }

    #[test]
    fn single_range_with_removals() {
        let ranges = find_ranges(
            [1, 2, 3, 4].iter().into_fx_index_set(),
            [1, 2, 3, 4].iter().into_fx_index_set(),
            &[1, 2, 5, 3, 4].into_fx_index_set(),
            &[1, 2, 3, 4].into_fx_index_set(),
        );

        assert_eq!(ranges, vec![]);
    }

    #[test]
    fn two_ranges() {
        let ranges = find_ranges(
            [1, 2, 3, 4].iter().into_fx_index_set(),
            [3, 4, 1, 2].iter().into_fx_index_set(),
            &[1, 2, 3, 4].into_fx_index_set(),
            &[3, 4, 1, 2].into_fx_index_set(),
        );

        assert_eq!(
            ranges,
            vec![
                DiffOpMove {
                    from: 0,
                    to: 2,
                    len: 2,
                    is_dense: true
                },
                DiffOpMove {
                    from: 2,
                    to: 0,
                    len: 2,
                    is_dense: true
                }
            ]
        );
    }

    #[test]
    fn two_ranges_with_adds() {
        let ranges = find_ranges(
            [1, 2, 3, 4].iter().into_fx_index_set(),
            [3, 4, 1, 2].iter().into_fx_index_set(),
            &[1, 2, 3, 4].into_fx_index_set(),
            &[3, 4, 5, 1, 6, 2].into_fx_index_set(),
        );

        assert_eq!(
            ranges,
            vec![
                DiffOpMove {
                    from: 0,
                    to: 3,
                    len: 2,
                    is_dense: false,
                },
                DiffOpMove {
                    from: 2,
                    to: 0,
                    len: 2,
                    is_dense: true
                }
            ]
        );
    }
    #[test]
    fn two_ranges_with_removals() {
        let ranges = find_ranges(
            [1, 2, 3, 4].iter().into_fx_index_set(),
            [3, 4, 1, 2].iter().into_fx_index_set(),
            &[1, 5, 2, 6, 3, 4].into_fx_index_set(),
            &[3, 4, 1, 2].into_fx_index_set(),
        );

        assert_eq!(
            ranges,
            vec![
                DiffOpMove {
                    from: 0,
                    to: 2,
                    len: 2,
                    is_dense: true,
                },
                DiffOpMove {
                    from: 4,
                    to: 0,
                    len: 2,
                    is_dense: true
                }
            ]
        );
    }

    #[test]
    fn remove_ranges_that_did_not_move() {
        // Here, 'C' doesn't change
        let ranges = find_ranges(
            ['A', 'B', 'C', 'D'].iter().into_fx_index_set(),
            ['B', 'D', 'C', 'A'].iter().into_fx_index_set(),
            &['A', 'B', 'C', 'D'].into_fx_index_set(),
            &['B', 'D', 'C', 'A'].into_fx_index_set(),
        );

        assert_eq!(
            ranges,
            vec![
                DiffOpMove {
                    from: 0,
                    to: 3,
                    len: 1,
                    is_dense: true,
                },
                DiffOpMove {
                    from: 1,
                    to: 0,
                    len: 1,
                    is_dense: true
                },
                DiffOpMove {
                    from: 3,
                    to: 1,
                    len: 1,
                    is_dense: true
                },
            ]
        );

        // Now we're going to to the same as above, just with more items
        //
        // A = 1
        // B = 2, 3
        // C = 4, 5, 6
        // D = 7, 8, 9, 0

        let ranges = find_ranges(
            //A B     C        D
            [1, 2, 3, 4, 5, 6, 7, 8, 9, 0].iter().into_fx_index_set(),
            //B    D           C        A
            [2, 3, 7, 8, 9, 0, 4, 5, 6, 1].iter().into_fx_index_set(),
            //A  B     C        D
            &[1, 2, 3, 4, 5, 6, 7, 8, 9, 0].into_fx_index_set(),
            //B     D           C        A
            &[2, 3, 7, 8, 9, 0, 4, 5, 6, 1].into_fx_index_set(),
        );

        assert_eq!(
            ranges,
            vec![
                DiffOpMove {
                    from: 0,
                    to: 9,
                    len: 1,
                    is_dense: true,
                },
                DiffOpMove {
                    from: 1,
                    to: 0,
                    len: 2,
                    is_dense: true
                },
                DiffOpMove {
                    from: 6,
                    to: 2,
                    len: 4,
                    is_dense: true
                },
            ]
        );
    }
}

#[cfg(test)]
mod optimize_moves {
    use super::*;

    #[test]
    fn swap() {
        let mut moves = vec![
            DiffOpMove {
                from: 0,
                to: 6,
                len: 2,
                ..Default::default()
            },
            DiffOpMove {
                from: 6,
                to: 0,
                len: 7,
                ..Default::default()
            },
        ];

        optimize_moves(&mut moves);

        assert_eq!(
            moves,
            vec![DiffOpMove {
                from: 0,
                to: 6,
                len: 2,
                ..Default::default()
            }]
        );
    }
}
