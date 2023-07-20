#[cfg(not(all(target_arch = "wasm32", feature = "web")))]
use crate::hydration::HydrationKey;
use crate::{hydration::HydrationCtx, Comment, CoreComponent, IntoView, View};
use leptos_reactive::{as_child_of_current_owner, Disposer};
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

#[cfg(all(target_arch = "wasm32", feature = "web"))]
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
    disposer: Disposer,
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
    fn new(disposer: Disposer, child: View) -> Self {
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
            disposer,
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
    EF: Fn(T) -> N + 'static,
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
    EF: Fn(T) -> N + 'static,
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
    EF: Fn(T) -> N + 'static,
    N: IntoView + 'static,
    KF: Fn(&T) -> K + 'static,
    K: Eq + Hash + 'static,
    T: 'static,
{
    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(level = "info", name = "<Each />", skip_all)
    )]
    fn into_view(self) -> crate::View {
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

        let each_fn = as_child_of_current_owner(each_fn);

        #[cfg(all(target_arch = "wasm32", feature = "web"))]
        create_effect(move |prev_hash_run: Option<HashRun<FxIndexSet<K>>>| {
            let mut children_borrow = children.borrow_mut();

            #[cfg(all(target_arch = "wasm32", feature = "web"))]
            let opening = if let Some(Some(child)) = children_borrow.get(0) {
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
                        #[cfg(all(target_arch = "wasm32", feature = "web"))]
                        &opening,
                        #[cfg(all(target_arch = "wasm32", feature = "web"))]
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
                let (child, disposer) = each_fn(item);
                let each_item = EachItem::new(disposer, child.into_view());

                #[cfg(all(target_arch = "wasm32", feature = "web"))]
                {
                    _ = fragment.append_child(&each_item.get_mountable_node());
                }

                children_borrow.push(Some(each_item));
            }

            #[cfg(all(target_arch = "wasm32", feature = "web"))]
            closing
                .unchecked_ref::<web_sys::Element>()
                .before_with_node_1(&fragment)
                .expect("before to not err");

            HashRun(hashed_items)
        });

        #[cfg(not(all(target_arch = "wasm32", feature = "web")))]
        {
            *component.children.borrow_mut() = (items_fn)()
                .into_iter()
                .map(|child| {
                    let (item, disposer) = each_fn(child);
                    Some(EachItem::new(disposer, item.into_view()))
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
#[cfg(all(target_arch = "wasm32", feature = "web"))]
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
    let removed = from.difference(to);

    let remove_cmds = removed
        .clone()
        .map(|k| from.get_full(k).unwrap().0)
        .map(|idx| DiffOpRemove { at: idx });

    // Get added items
    let added = to.difference(from);

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

    let mut diff = Diff {
        removed: remove_cmds.collect(),
        items_to_move: move_cmds.iter().map(|range| range.len).sum(),
        moved: move_cmds,
        added: add_cmds.collect(),
        clear: false,
    };

    apply_opts(from, to, &mut diff);

    #[cfg(test)]
    {
        let mut adds_sorted = diff.added.clone();
        adds_sorted.sort_unstable_by_key(|add| add.at);

        assert_eq!(diff.added, adds_sorted, "adds must be sorted");

        let mut moves_sorted = diff.moved.clone();
        moves_sorted.sort_unstable_by_key(|move_| move_.to);

        assert_eq!(diff.moved, moves_sorted, "moves must be sorted by `to`");
    }

    diff
}

/// Builds and returns the ranges of items that need to
/// move sorted by `to`.
#[cfg(all(target_arch = "wasm32", feature = "web"))]
fn find_ranges<K: Eq + Hash>(
    from_moved: FxIndexSet<&K>,
    to_moved: FxIndexSet<&K>,
    from: &FxIndexSet<K>,
    to: &FxIndexSet<K>,
) -> Vec<DiffOpMove> {
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
        }
        // We're done with this range, start a new one
        else {
            ranges.push(std::mem::take(&mut range));

            range.from = from.get_index_of(k).unwrap();
            range.to = to.get_index_of(k).unwrap();
        }

        prev_to_moved_index = to_moved_index;
    }

    ranges.push(std::mem::take(&mut range));

    // We need to remove ranges that didn't move relative to each other
    // as well as marking items that don't need to move in the DOM
    let mut to_ranges = ranges.clone();
    to_ranges.sort_unstable_by_key(|range| range.to);

    let mut filtered_ranges = vec![];

    let to_ranges_len = to_ranges.len();

    for (i, range) in to_ranges.into_iter().enumerate() {
        if range != ranges[i] {
            filtered_ranges.push(range);
        }
        // The item did move, just not in the DOM
        else if range.from != range.to {
            filtered_ranges.push(DiffOpMove {
                move_in_dom: false,
                ..range
            });
        } else if to_ranges_len > 2 {
            // TODO: Remove this else case...this is one of the biggest
            // optimizations we can do, but we're skipping this right now
            // until we figure out a way to handle moving around ranges
            // that did not move
            filtered_ranges.push(range);
        }
    }

    filtered_ranges
}

#[cfg(all(target_arch = "wasm32", feature = "web"))]
fn apply_opts<K: Eq + Hash>(
    from: &FxIndexSet<K>,
    to: &FxIndexSet<K>,
    cmds: &mut Diff,
) {
    optimize_moves(&mut cmds.moved);

    // We can optimize the case of replacing all items
    if !from.is_empty()
        && !to.is_empty()
        && cmds.removed.len() == from.len()
        && cmds.moved.is_empty()
    {
        cmds.clear = true;
        cmds.removed.clear();

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
}

#[cfg(all(target_arch = "wasm32", feature = "web"))]
fn optimize_moves(moves: &mut Vec<DiffOpMove>) {
    if moves.is_empty() || moves.len() == 1 {
        // Do nothing
    }
    // This is the easiest optimal move case, which is to
    // simply swap the 2 ranges. We only need to move the range
    // that is smallest.
    else if moves.len() == 2 {
        if moves[1].len < moves[0].len {
            moves[0].move_in_dom = false;
        } else {
            moves[1].move_in_dom = false;
        }
    }
    // Interestingly enoughs, there are NO configuration that are possible
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
        // around it.
        let move_ = moves.iter_mut().max_by_key(|move_| move_.len).unwrap();

        move_.move_in_dom = false;
    }
}

#[cfg(all(target_arch = "wasm32", feature = "web"))]
#[derive(Debug, Default, PartialEq, Eq)]
struct Diff {
    removed: Vec<DiffOpRemove>,
    moved: Vec<DiffOpMove>,
    items_to_move: usize,
    added: Vec<DiffOpAdd>,
    clear: bool,
}

#[cfg(all(target_arch = "wasm32", feature = "web"))]
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

#[cfg(all(target_arch = "wasm32", feature = "web"))]
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

#[cfg(all(target_arch = "wasm32", feature = "web"))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct DiffOpAdd {
    at: usize,
    mode: DiffOpAddMode,
}

#[cfg(all(target_arch = "wasm32", feature = "web"))]
#[derive(Debug, PartialEq, Eq)]
struct DiffOpRemove {
    at: usize,
}

#[cfg(all(target_arch = "wasm32", feature = "web"))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DiffOpAddMode {
    Normal,
    Append,
    // Todo
    _Prepend,
}

#[cfg(all(target_arch = "wasm32", feature = "web"))]
impl Default for DiffOpAddMode {
    fn default() -> Self {
        Self::Normal
    }
}

#[cfg(all(target_arch = "wasm32", feature = "web"))]
fn apply_diff<T, EF, V>(
    opening: &web_sys::Node,
    closing: &web_sys::Node,
    diff: Diff,
    children: &mut Vec<Option<EachItem>>,
    mut items: Vec<Option<T>>,
    each_fn: &EF,
) where
    EF: Fn(T) -> (V, Disposer),
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
        let (item, disposer) = each_fn(items[at].take().unwrap());
        let each_item = EachItem::new(disposer, item.into_view());

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

// #[cfg(test)]
// mod test_utils {
//     use super::*;

//     pub trait IntoFxIndexSet<K> {
//         fn into_fx_index_set(self) -> FxIndexSet<K>;
//     }

//     impl<T, K> IntoFxIndexSet<K> for T
//     where
//         T: IntoIterator<Item = K>,
//         K: Eq + Hash,
//     {
//         fn into_fx_index_set(self) -> FxIndexSet<K> {
//             self.into_iter().collect()
//         }
//     }
// }

// #[cfg(test)]
// use test_utils::*;

// #[cfg(test)]
// mod find_ranges {
//     use super::*;

//     // Single range tests will be empty because of removing ranges
//     // that didn't move
//     #[test]
//     fn single_range() {
//         let ranges = find_ranges(
//             [1, 2, 3, 4].iter().into_fx_index_set(),
//             [1, 2, 3, 4].iter().into_fx_index_set(),
//             &[1, 2, 3, 4].into_fx_index_set(),
//             &[1, 2, 3, 4].into_fx_index_set(),
//         );

//         assert_eq!(ranges, vec![]);
//     }

//     #[test]
//     fn single_range_with_adds() {
//         let ranges = find_ranges(
//             [1, 2, 3, 4].iter().into_fx_index_set(),
//             [1, 2, 3, 4].iter().into_fx_index_set(),
//             &[1, 2, 3, 4].into_fx_index_set(),
//             &[1, 2, 5, 3, 4].into_fx_index_set(),
//         );

//         assert_eq!(ranges, vec![]);
//     }

//     #[test]
//     fn single_range_with_removals() {
//         let ranges = find_ranges(
//             [1, 2, 3, 4].iter().into_fx_index_set(),
//             [1, 2, 3, 4].iter().into_fx_index_set(),
//             &[1, 2, 5, 3, 4].into_fx_index_set(),
//             &[1, 2, 3, 4].into_fx_index_set(),
//         );

//         assert_eq!(ranges, vec![]);
//     }

//     #[test]
//     fn two_ranges() {
//         let ranges = find_ranges(
//             [1, 2, 3, 4].iter().into_fx_index_set(),
//             [3, 4, 1, 2].iter().into_fx_index_set(),
//             &[1, 2, 3, 4].into_fx_index_set(),
//             &[3, 4, 1, 2].into_fx_index_set(),
//         );

//         assert_eq!(
//             ranges,
//             vec![
//                 DiffOpMove {
//                     from: 2,
//                     to: 0,
//                     len: 2,
//                     move_in_dom: true,
//                 },
//                 DiffOpMove {
//                     from: 0,
//                     to: 2,
//                     len: 2,
//                     move_in_dom: true,
//                 },
//             ]
//         );
//     }

//     #[test]
//     fn two_ranges_with_adds() {
//         let ranges = find_ranges(
//             [1, 2, 3, 4].iter().into_fx_index_set(),
//             [3, 4, 1, 2].iter().into_fx_index_set(),
//             &[1, 2, 3, 4].into_fx_index_set(),
//             &[3, 4, 5, 1, 6, 2].into_fx_index_set(),
//         );

//         assert_eq!(
//             ranges,
//             vec![
//                 DiffOpMove {
//                     from: 2,
//                     to: 0,
//                     len: 2,
//                 },
//                 DiffOpMove {
//                     from: 0,
//                     to: 3,
//                     len: 2,
//                 },
//             ]
//         );
//     }
//     #[test]
//     fn two_ranges_with_removals() {
//         let ranges = find_ranges(
//             [1, 2, 3, 4].iter().into_fx_index_set(),
//             [3, 4, 1, 2].iter().into_fx_index_set(),
//             &[1, 5, 2, 6, 3, 4].into_fx_index_set(),
//             &[3, 4, 1, 2].into_fx_index_set(),
//         );

//         assert_eq!(
//             ranges,
//             vec![
//                 DiffOpMove {
//                     from: 4,
//                     to: 0,
//                     len: 2,
//                 },
//                 DiffOpMove {
//                     from: 0,
//                     to: 2,
//                     len: 2,
//                 },
//             ]
//         );
//     }

//     #[test]
//     fn remove_ranges_that_did_not_move() {
//         // Here, 'C' doesn't change
//         let ranges = find_ranges(
//             ['A', 'B', 'C', 'D'].iter().into_fx_index_set(),
//             ['B', 'D', 'C', 'A'].iter().into_fx_index_set(),
//             &['A', 'B', 'C', 'D'].into_fx_index_set(),
//             &['B', 'D', 'C', 'A'].into_fx_index_set(),
//         );

//         assert_eq!(
//             ranges,
//             vec![
//                 DiffOpMove {
//                     from: 1,
//                     to: 0,
//                     len: 1,
//                 },
//                 DiffOpMove {
//                     from: 3,
//                     to: 1,
//                     len: 1,
//                 },
//                 DiffOpMove {
//                     from: 0,
//                     to: 3,
//                     len: 1,
//                 },
//             ]
//         );

//         // Now we're going to to the same as above, just with more items
//         //
//         // A = 1
//         // B = 2, 3
//         // C = 4, 5, 6
//         // D = 7, 8, 9, 0

//         let ranges = find_ranges(
//             //A B     C        D
//             [1, 2, 3, 4, 5, 6, 7, 8, 9, 0].iter().into_fx_index_set(),
//             //B    D           C        A
//             [2, 3, 7, 8, 9, 0, 4, 5, 6, 1].iter().into_fx_index_set(),
//             //A  B     C        D
//             &[1, 2, 3, 4, 5, 6, 7, 8, 9, 0].into_fx_index_set(),
//             //B     D           C        A
//             &[2, 3, 7, 8, 9, 0, 4, 5, 6, 1].into_fx_index_set(),
//         );

//         assert_eq!(
//             ranges,
//             vec![
//                 DiffOpMove {
//                     from: 1,
//                     to: 0,
//                     len: 2,
//                 },
//                 DiffOpMove {
//                     from: 6,
//                     to: 2,
//                     len: 4,
//                 },
//                 DiffOpMove {
//                     from: 0,
//                     to: 9,
//                     len: 1,
//                 },
//             ]
//         );
//     }
// }

// #[cfg(test)]
// mod optimize_moves {
//     use super::*;

//     #[test]
//     fn swap() {
//         let mut moves = vec![
//             DiffOpMove {
//                 from: 0,
//                 to: 6,
//                 len: 2,
//                 ..Default::default()
//             },
//             DiffOpMove {
//                 from: 6,
//                 to: 0,
//                 len: 7,
//                 ..Default::default()
//             },
//         ];

//         optimize_moves(&mut moves);

//         assert_eq!(
//             moves,
//             vec![DiffOpMove {
//                 from: 0,
//                 to: 6,
//                 len: 2,
//                 ..Default::default()
//             }]
//         );
//     }
// }

// #[cfg(test)]
// mod add_or_move {
//     use super::*;

//     #[test]
//     fn simple_range() {
//         let cmds = AddOrMove::from_diff(&Diff {
//             moved: vec![DiffOpMove {
//                 from: 0,
//                 to: 0,
//                 len: 3,
//             }],
//             ..Default::default()
//         });

//         assert_eq!(
//             cmds,
//             vec![
//                 DiffOpMove {
//                     from: 0,
//                     to: 0,
//                     len: 1,
//                 },
//                 DiffOpMove {
//                     from: 1,
//                     to: 1,
//                     len: 1,
//                 },
//                 DiffOpMove {
//                     from: 2,
//                     to: 2,
//                     len: 1,
//                 },
//             ]
//         );
//     }

//     #[test]
//     fn range_with_add() {
//         let cmds = AddOrMove::from_diff(&Diff {
//             moved: vec![DiffOpMove {
//                 from: 0,
//                 to: 0,
//                 len: 3,
//                 move_in_dom: true,
//             }],
//             added: vec![DiffOpAdd {
//                 at: 2,
//                 ..Default::default()
//             }],
//             ..Default::default()
//         });

//         assert_eq!(
//             cmds,
//             vec![
//                 AddOrMove::Move(DiffOpMove {
//                     from: 0,
//                     to: 0,
//                     len: 1,
//                     move_in_dom: true,
//                 }),
//                 AddOrMove::Move(DiffOpMove {
//                     from: 1,
//                     to: 1,
//                     len: 1,
//                     move_in_dom: true,
//                 }),
//                 AddOrMove::Add(DiffOpAdd {
//                     at: 2,
//                     ..Default::default()
//                 }),
//                 AddOrMove::Move(DiffOpMove {
//                     from: 3,
//                     to: 3,
//                     len: 1,
//                     move_in_dom: true,
//                 }),
//             ]
//         );
//     }
// }

// #[cfg(test)]
// mod diff {
//     use super::*;

//     #[test]
//     fn only_adds() {
//         let diff =
//             diff(&[].into_fx_index_set(), &[1, 2, 3].into_fx_index_set());

//         assert_eq!(
//             diff,
//             Diff {
//                 added: vec![
//                     DiffOpAdd {
//                         at: 0,
//                         mode: DiffOpAddMode::Append
//                     },
//                     DiffOpAdd {
//                         at: 1,
//                         mode: DiffOpAddMode::Append
//                     },
//                     DiffOpAdd {
//                         at: 2,
//                         mode: DiffOpAddMode::Append
//                     },
//                 ],
//                 ..Default::default()
//             }
//         );
//     }

//     #[test]
//     fn only_removes() {
//         let diff =
//             diff(&[1, 2, 3].into_fx_index_set(), &[3].into_fx_index_set());

//         assert_eq!(
//             diff,
//             Diff {
//                 removed: vec![DiffOpRemove { at: 0 }, DiffOpRemove { at: 1 }],
//                 ..Default::default()
//             }
//         );
//     }

//     #[test]
//     fn adds_with_no_move() {
//         let diff =
//             diff(&[3].into_fx_index_set(), &[1, 2, 3].into_fx_index_set());

//         assert_eq!(
//             diff,
//             Diff {
//                 added: vec![
//                     DiffOpAdd {
//                         at: 0,
//                         ..Default::default()
//                     },
//                     DiffOpAdd {
//                         at: 1,
//                         ..Default::default()
//                     },
//                 ],
//                 ..Default::default()
//             }
//         );
//     }
// }
