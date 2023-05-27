#[cfg(not(all(target_arch = "wasm32", feature = "web")))]
use crate::hydration::HydrationKey;
use crate::{hydration::HydrationCtx, Comment, CoreComponent, IntoView, View};
use leptos_reactive::Scope;
use std::{borrow::Cow, cell::RefCell, fmt, hash::Hash, ops::Deref, rc::Rc};
#[cfg(all(target_arch = "wasm32", feature = "web"))]
use web_imports::*;

#[cfg(all(target_arch = "wasm32", feature = "web"))]
mod web_imports {
    pub(super) use crate::{
        mount_child, prepare_to_move, MountKind, Mountable, RANGE,
    };
    pub(super) use cfg_if::cfg_if;
    pub(super) use drain_filter_polyfill::VecExt as VecDrainFilterExt;
    pub(super) use leptos_reactive::create_effect;
    pub(super) use once_cell::unsync::OnceCell;
    pub(super) use wasm_bindgen::JsCast;
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
        crate::log!("get_next_closest_mounted_sibling {self:?} \n\n start_at = {start_at}");
        self[start_at..]
            .iter()
            .find_map(|s| s.as_ref().map(|s| {
                //crate::log!("checking {s:?}");
                s.get_opening_node()
            }))
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

        // TODO fix this -- something caused it to break moves?
        let needs_closing = true; // !matches!(child, View::Element(_));

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
        {
            create_effect(
                cx,
                move |prev_hash_run: Option<HashRun<FxIndexSet<K>>>| {
                    let mut children_borrow = children.borrow_mut();

                    let opening =
                        if let Some(Some(child)) = children_borrow.get(0) {
                            child.get_opening_node()
                        } else {
                            closing.clone()
                        };

                    let items_iter = items_fn().into_iter();

                    let (capacity, _) = items_iter.size_hint();
                    let mut hashed_items = FxIndexSet::default();
                    hashed_items.reserve(capacity);

                    if let Some(HashRun(prev_hash_run)) = prev_hash_run {
                        if !prev_hash_run.is_empty() {
                            // Compiler will optimize this, so no need to use
                            // `Vec::with_capacity()` here
                            let items = items_iter
                                .map(|item| {
                                    hashed_items.insert(key_fn(&item));

                                    Some(item)
                                })
                                .collect::<Vec<_>>();

                            let mut cmds = diff(&prev_hash_run, &hashed_items);

                            apply_opts(
                                &prev_hash_run,
                                &hashed_items,
                                &mut cmds,
                            );

                            apply_cmds(
                                cx,
                                &opening,
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

                    let fragment = crate::document().create_document_fragment();

                    for item in items_iter {
                        hashed_items.insert(key_fn(&item));
                        let (each_item, _) = cx.run_child_scope(|cx| {
                            EachItem::new(cx, each_fn(cx, item).into_view(cx))
                        });

                        fragment
                            .append_child(&each_item.get_mountable_node())
                            .unwrap();

                        children_borrow.push(Some(each_item));
                    }

                    closing
                        .unchecked_ref::<web_sys::Element>()
                        .before_with_node_1(&fragment)
                        .expect("before to not err");

                    HashRun(hashed_items)
                },
            );
        }

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

    // Create index set of `from` and `to` values that contains
    // only moved items
    let mut moved_from = from.intersection(&to).collect::<FxIndexSet<_>>();
    let mut moved_to = to.intersection(&from).collect::<FxIndexSet<_>>();

    // Guaranteed to have the same number of elements. This
    // assertion is not needed, but is an aid to the reader
    debug_assert_eq!(moved_from.len(), moved_to.len());

    // Filter items that have not moved
    //
    // `FxIndexSet` does not have a `drain_filter`, which
    // means that to remove items, we would incur O(n^n)
    // because we'd need to shift all elements at the same
    // time. Therefore, it's faster to create a new set
    // that copies over the references we want
    let moved_from = moved_from
        .into_iter()
        .enumerate()
        .filter(|(i, k)| *i != moved_to.get_index_of(*k).unwrap())
        .map(|(_, k)| k)
        .collect::<FxIndexSet<_>>();
    let moved_to = moved_to
        .intersection(&moved_from)
        .map(|k| *k)
        .collect::<FxIndexSet<_>>();

    let moved_from = moved_from;
    let moved_to = moved_to;

    // Find ranges to optimize moves later
    let ranges = find_ranges(from, to, &moved_from, &moved_to);

    // Hard part's over, now we just need to apply optimizations

    // TODO: update `Diff.moved` to use `SmallVec`
    let mut move_cmds = ranges.into_iter().collect();

    Diff {
        removed: removed_cmds.collect(),
        moved: move_cmds,
        added: added_cmds.collect(),
        clear: false,
    }
}

#[cfg(any(test, all(target_arch = "wasm32", feature = "web")))]
fn apply_opts<K: Eq + Hash>(
    from: &FxIndexSet<K>,
    to: &FxIndexSet<K>,
    cmds: &mut Diff,
) {
    // We can optimize the case of replacing all items
    if !from.is_empty() && !to.is_empty() && cmds.removed.len() == from.len() {
        debug_assert!(cmds.moved.is_empty());

        cmds.clear = true;

        cmds.removed.clear();
        cmds.added
            .iter_mut()
            .for_each(|op| op.mode = DiffOpAddMode::Append);

        return;
    }

    // // We can optimize appends.
    // if !cmds.added.is_empty()
    //     && cmds.moved.is_empty()
    //     && cmds.removed.is_empty()
    //     && cmds.added[0].at >= from.len()
    // {
    //     cmds.added
    //         .iter_mut()
    //         .for_each(|op| op.mode = DiffOpAddMode::Append);
    // }

    // Move optimizations
    if cmds.moved.len() == 3 {
        // This one's a little hard to explain, but basically,
        // we want to hand-optimize the case where there are
        // exactly 3 ranges to minimize the number of moves.
        // We can do this for 4 or more, but I think 3 is a
        // happy ground for now. We also don't need to do 0
        // or 1, because the "filter items that didn't move"
        // step above makes these cases impossible. We also
        // don't need to do 2 because the moves are already
        // minimal, i.e., swapping.
        //
        // But it goes like this, we have 3 ranges:
        // from: [A, B, C]
        //
        // There are 3 factorial (6) ways to arrange `from` TO
        // to get to `to`:
        //
        // 1. [A, B, C]
        // 2. [A, C, B]
        // 3. [B, A, C]
        // 4. [B, C, A]
        // 5. [C, A, B]
        // 6. [C, B, A]
        //
        // That's a lot of cases, however, here's a trick we can
        // use to get this list down to just 2. Remember the
        // "filter items the didn't move" step? We can exclude
        // any combination that has any of the column letters unmoved.
        //
        // This means
        //
        //    [A, B, C] // original, removing with respect to A
        // 1. [A, B, C]
        // 2. [A, C, B]
        //
        //    [A, B, C] // original, removing with respect to B
        // 1. [A, B, C]
        // 6. [C, B, A]
        //
        //    [A, B, C] // original, removing with respect to C
        // 1. [A, B, C]
        // 3. [B, A, C]
        //
        //  1, 2, 3, and 6 are impossible cases, which leaves us
        // with only needing to worry about:
        //
        // 4. [B, C, A]
        // 5. [C, A, B]
        //
        // In the case of 4, they all move, so no point on doing
        // anything special.
        //
        // For 5, C should be the only one to move.

        const A: usize = 0;
        const B: usize = 1;
        const C: usize = 2;

        // Are we arranged in the [C, A, B] configuration?
        if cmds.moved[C].to < cmds.moved[A].to
            && cmds.moved[A].to < cmds.moved[B].to
        {
            cmds.moved.remove(A);
            cmds.moved.remove(B);
        }
    }
}

#[derive(Debug, Default, PartialEq, Eq)]
#[allow(unused)]
struct Diff {
    removed: Vec<DiffOpRemove>,
    moved: Vec<DiffOpMove>,
    added: Vec<DiffOpAdd>,
    clear: bool,
}

#[derive(Default, Debug, PartialEq, Eq)]
#[allow(unused)]
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

#[derive(Default, Debug, PartialEq, Eq)]
#[allow(unused)]
struct DiffOpAdd {
    at: usize,
    mode: DiffOpAddMode,
}

#[derive(Debug, PartialEq, Eq)]
#[allow(unused)]
struct DiffOpRemove {
    at: usize,
}

#[derive(Debug, PartialEq, Eq)]
#[allow(unused)]
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
fn apply_cmds<T, EF, N>(
    cx: Scope,
    opening: &web_sys::Node,
    closing: &web_sys::Node,
    mut cmds: Diff,
    children: &mut Vec<Option<EachItem>>,
    mut items: Vec<Option<T>>,
    each_fn: &EF,
) where
    EF: Fn(Scope, T) -> N,
    N: IntoView,
{
    let range = RANGE.with(|range| (*range).clone());
    let parent = closing
        .parent_node()
        .expect("`Each` to have a parent node")
        .unchecked_into::<web_sys::Element>();

    // Resize children if needed
    if cmds.added.len().checked_sub(cmds.removed.len()).is_some() {
        let target_size = children.len()
            + (cmds.added.len() as isize - cmds.removed.len() as isize)
                as usize;

        children.resize_with(target_size, || None);
    }

    // The order of cmds needs to be:
    // 1. Clear
    // 2. Removed
    // 3. Dense moves
    // 4. Non-dense moves and adds must be applied together interleaved
    if cmds.clear {
        cmds.removed.clear();

        if opening.previous_sibling().is_none()
            && closing.next_sibling().is_none()
        {
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

    for DiffOpRemove { at } in cmds.removed {
        let item_to_remove = std::mem::take(&mut children[at]).unwrap();

        item_to_remove.prepare_for_move();
    }

    crate::log!("adds = {:?}", cmds.added);

    let mut added_iter = cmds.added.into_iter();
    let mut unrelated_adds = Vec::new();

    crate::log!("moves = {:?}", cmds.moved);

    for DiffOpMove {
        from,
        len,
        to,
        is_dense,
    } in cmds.moved
    {
        // Since we will be moving ranges, we need to
        // check to see if it's possible that an added
        // item was inserted in the middle of the range.
        // In this case, what we're going to do is
        // move each child and insert new item(s).
        // If we don't do it this way, but rather move
        // everything and then insert the new children,
        // we would need to incur the cost of inserting
        // in the middle of a Vec for each item, this is
        // no bueno.
        if is_dense {
            // TODO optimize for case of a move of len 1 of a single Element?
            range.set_start_before(
                &children[from].as_ref().unwrap().get_opening_node(),
            );
            range.set_end_before(
                &children[from + len - 1]
                    .as_ref()
                    .unwrap()
                    .get_closing_node(),
            );

            let contents = range.extract_contents().unwrap();

            let opening = children
                .get_next_closest_mounted_sibling(to + 1, closing.to_owned());

            opening
                .unchecked_ref::<web_sys::Element>()
                .before_with_node_1(&contents);

            // TODO update children... so that subsequent adds are against correct index
        }
        // non-dense moves
        else {
            let move_range = (from..from + len);
            // there may be additional adds that are not the one
            // inserted into this move, lets iterate through them
            while let Some(next_add) = added_iter.next() {
                let next_add_at = next_add.at;
                if move_range.contains(&next_add_at) {
                    // do the non-dense move
                    crate::log!("add {next_add:?} during this move");

                    // break out so we stop consuming iterator of adds
                    break;
                } else {
                    crate::log!("unrelated add {next_add:?}");
                    unrelated_adds.push(next_add);
                }
            }
        }
    }

    for DiffOpAdd { at, mode } in added_iter.chain(unrelated_adds) {
        add_item(cx, &mut items, children, each_fn, at, mode, closing);
    }

    // Now, remove the holes that might have been left from removing
    // items
    #[allow(unstable_name_collisions)]
    children.drain_filter(|c| c.is_none());
}

#[cfg(all(target_arch = "wasm32", feature = "web"))]
fn add_item<T, EF, N>(
    cx: Scope,
    items: &mut Vec<Option<T>>,
    children: &mut Vec<Option<EachItem>>,
    each_fn: &EF,
    at: usize,
    mode: DiffOpAddMode,
    closing: &web_sys::Node
) where EF: Fn(Scope, T) -> N, N: IntoView, {
    let item = items[at].take().unwrap();

    let (each_item, _) = cx.run_child_scope(|cx| {
        let child = each_fn(cx, item).into_view(cx);
        EachItem::new(cx, child)
    });

    match mode {
        DiffOpAddMode::Normal => {
            let opening = children
                .get_next_closest_mounted_sibling(at, closing.to_owned());
            crate::log!("adding at {at} with mode {mode:?} => {each_item:?} \n\nbefore {:?}\n\nchildren are {children:?}", opening.text_content());
            mount_child(MountKind::Before(&opening), &each_item);

            // shift subsequent items, so that subsequent adds are inserted in the right place
            let mut at = at;
            let mut old = std::mem::replace(&mut children[at], Some(each_item));
            while let Some(displaced) = old {
                old = std::mem::replace(&mut children[at + 1], Some(displaced));
                at += 1;
            }
        }
        DiffOpAddMode::Append => {
            mount_child(MountKind::Before(closing), &each_item);
            children[at] = Some(each_item);
        }
        DiffOpAddMode::_Prepend => todo!(),
    }
}

#[cfg(any(test, all(target_arch = "wasm32", feature = "web")))]
fn find_ranges<K: Eq + Hash>(
    from: &FxIndexSet<K>,
    to: &FxIndexSet<K>,
    moved_from: &FxIndexSet<&K>,
    moved_to: &FxIndexSet<&K>,
) -> smallvec::SmallVec<[DiffOpMove; 4]> {
    debug_assert_eq!(moved_from.len(), moved_to.len());
    debug_assert!(to.len() >= moved_to.len());

    let mut ranges: smallvec::SmallVec<[DiffOpMove; 4]> = smallvec::smallvec![];

    let mut range = DiffOpMove::default();

    for (i, k) in moved_from.iter().enumerate() {
        // Starting a new range
        if range.len == 0 {
            range.from = i;
            range.to = moved_to.get_index_of(k).unwrap();
            range.len = 1;
            range.is_dense = true;
        }
        // Is the current `k` contiguous with respect to the
        // last one?
        else if range.to + range.len == moved_to.get_index_of(k).unwrap() {
            range.len += 1;

            // We need to check for density
            if range.is_dense {
                range.is_dense =
                    to.get_index_of(*k).unwrap() == range.to + range.len - 1;
            }

            if i == moved_from.len() - 1 {
                ranges.push(std::mem::take(&mut range));
            }
        }
        // Otherwise, we're done with the range
        else {
            ranges.push(std::mem::take(&mut range));

            range.from = i;
            range.to = moved_to.get_index_of(k).unwrap();
            range.len = 1;
            range.is_dense = true;
        }
    }

    // Now, we need to map the ranges into the
    // unnormalized `from`/`to` index space
    ranges.iter_mut().for_each(|range| {
        let k = moved_from.get_index(range.from).unwrap();

        range.from = from.get_index_of(*k).unwrap();
        range.to = to.get_index_of(*k).unwrap();
    });

    ranges
}

#[cfg(test)]
mod testing_utils {
    use super::*;

    pub trait IntoFxIndexSet<K>: Sized {
        fn into_fx_index_set(self) -> FxIndexSet<K>;
    }

    impl<T, I> IntoFxIndexSet<I> for T
    where
        T: Sized + IntoIterator<Item = I>,
        I: Eq + Hash,
    {
        fn into_fx_index_set(self) -> FxIndexSet<I> {
            self.into_iter().collect()
        }
    }
}

#[cfg(test)]
mod find_ranges {
    use super::{testing_utils::IntoFxIndexSet, *};

    #[test]
    fn two_ranges() {
        let ranges = find_ranges(
            &[0, 1, 2, 3].into_fx_index_set(),
            &[2, 3, 0, 1].into_fx_index_set(),
            &[0, 1, 2, 3].iter().by_ref().into_fx_index_set(),
            &[2, 3, 0, 1].iter().into_fx_index_set(),
        );

        assert_eq!(
            ranges,
            smallvec::smallvec![
                Range {
                    from: 0,
                    len: 2,
                    to: 2,
                },
                Range {
                    from: 2,
                    len: 2,
                    to: 0,
                },
            ] as smallvec::SmallVec<[Range; 4]>
        );
    }

    fn three_single_ranges() {
        let ranges = find_ranges(
            &[0, 1, 2].into_fx_index_set(),
            &[2, 1, 0].into_fx_index_set(),
            &[0, 1, 2].iter().into_fx_index_set(),
            &[2, 1, 0].iter().into_fx_index_set(),
        );

        assert_eq!(
            ranges,
            smallvec::smallvec![
                Range {
                    from: 0,
                    len: 1,
                    to: 2,
                },
                Range {
                    from: 1,
                    len: 1,
                    to: 1,
                },
                Range {
                    from: 2,
                    len: 1,
                    to: 0,
                },
            ] as smallvec::SmallVec<[Range; 4]>
        );
    }

    fn three_ranges() {
        let ranges = find_ranges(
            &[0, 1, 2, 3, 4, 5].into_fx_index_set(),
            &[5, 1, 2, 3, 4, 0].into_fx_index_set(),
            &[0, 1, 2, 3, 4, 5].iter().into_fx_index_set(),
            &[5, 1, 2, 3, 4, 0].iter().into_fx_index_set(),
        );

        assert_eq!(
            ranges,
            smallvec::smallvec![
                Range {
                    from: 0,
                    len: 1,
                    to: 5,
                },
                Range {
                    from: 1,
                    len: 4,
                    to: 1,
                },
                Range {
                    from: 5,
                    len: 1,
                    to: 0,
                },
            ] as smallvec::SmallVec<[Range; 4]>
        );
    }
}

#[cfg(test)]
mod diff {
    use super::{testing_utils::IntoFxIndexSet, *};

    #[test]
    fn removes() {
        let diff = diff(
            &[0, 1, 2, 3, 4].into_fx_index_set(),
            &[0, 2].into_fx_index_set(),
        );

        assert_eq!(
            diff,
            Diff {
                removed: vec![
                    DiffOpRemove { at: 1 },
                    DiffOpRemove { at: 3 },
                    DiffOpRemove { at: 4 },
                ],
                ..Default::default()
            }
        );
    }

    #[test]
    fn adds() {
        let diff = diff(
            &[0, 2].into_fx_index_set(),
            &[0, 1, 2, 3, 4].into_fx_index_set(),
        );

        assert_eq!(
            diff,
            Diff {
                added: vec![
                    DiffOpAdd {
                        at: 1,
                        ..Default::default()
                    },
                    DiffOpAdd {
                        at: 3,
                        ..Default::default()
                    },
                    DiffOpAdd {
                        at: 4,
                        ..Default::default()
                    },
                ],
                ..Default::default()
            }
        );
    }

    #[test]
    fn adds_and_removes_dont_cause_moves() {
        let diff = diff(
            &[0, 1, 2, 3, 4, 5].into_fx_index_set(),
            &[1, 4, 7].into_fx_index_set(),
        );

        assert_eq!(
            diff,
            Diff {
                added: vec![DiffOpAdd {
                    at: 2,
                    ..Default::default()
                }],
                removed: vec![
                    DiffOpRemove { at: 0 },
                    DiffOpRemove { at: 2 },
                    DiffOpRemove { at: 3 },
                    DiffOpRemove { at: 5 },
                ],

                ..Default::default()
            }
        );
    }

    #[test]
    fn swap() {
        let diff = diff(
            &[0, 1, 2, 3].into_fx_index_set(),
            &[2, 3, 0, 1].into_fx_index_set(),
        );

        assert_eq!(
            diff,
            Diff {
                moved: vec![
                    DiffOpMove {
                        from: 0,
                        len: 2,
                        to: 2,
                    },
                    DiffOpMove {
                        from: 2,
                        len: 2,
                        to: 0,
                    },
                ],
                ..Default::default()
            }
        );
    }

    #[test]
    fn non_moves_are_filtered_out() {
        let diff = diff(
            &[0, 1, 2, 3, 4].into_fx_index_set(),
            &[3, 4, 2, 0, 1].into_fx_index_set(),
        );

        assert_eq!(
            diff,
            Diff {
                moved: vec![
                    DiffOpMove {
                        from: 0,
                        len: 2,
                        to: 3,
                    },
                    DiffOpMove {
                        from: 3,
                        len: 2,
                        to: 0,
                    },
                ],
                ..Default::default()
            }
        );
    }

    #[test]
    fn additions_and_removals_in_middle_of_range_dont_break_range() {
        let diff = diff(
            &[0, 1, 2, 3, 4, 5].into_fx_index_set(),
            &[3, 5, 0, 6, 1, 2].into_fx_index_set(),
        );

        assert_eq!(
            diff,
            Diff {
                added: vec![DiffOpAdd {
                    at: 3,
                    ..Default::default()
                }],
                removed: vec![DiffOpRemove { at: 4 }],
                moved: vec![
                    DiffOpMove {
                        from: 0,
                        len: 3,
                        to: 2,
                    },
                    DiffOpMove {
                        from: 3,
                        len: 2,
                        to: 0,
                    },
                ],
                ..Default::default()
            }
        );
    }
}

#[cfg(test)]
mod apply_opts {
    use super::{testing_utils::IntoFxIndexSet, *};

    #[test]
    fn replace_all_items() {
        let from = [0, 1, 2].into_fx_index_set();
        let to = [3, 4, 5].into_fx_index_set();

        let mut diff = diff(&from, &to);

        apply_opts(&from, &to, &mut diff);

        assert_eq!(
            diff,
            Diff {
                clear: true,
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

    fn c_a_b_moves_optimization() {
        let from = ['a', 'b', 'c'].into_fx_index_set();
        let to = ['c', 'a', 'b'].into_fx_index_set();

        let mut diff = diff(&from, &to);

        apply_opts(&from, &to, &mut diff);

        assert_eq!(
            diff,
            Diff {
                moved: vec![DiffOpMove {
                    from: 2,
                    len: 1,
                    to: 0,
                }],
                ..Default::default()
            }
        );
    }
}
