#[cfg(not(all(target_arch = "was32", feature = "web")))]
use crate::hydration::HydrationKey;
use crate::{hydration::HydrationCtx, Comment, CoreComponent, IntoView, View};
use leptos_reactive::Scope;
use std::{borrow::Cow, cell::RefCell, fmt, hash::Hash, ops::Deref, rc::Rc};
#[cfg(all(target_arch = "wasm32", feature = "web"))]
use web::*;

#[cfg(all(target_arch = "wasm32", feature = "web"))]
mod web {
    use crate::{mount_child, prepare_to_move, MountKind, Mountable, RANGE};
    use drain_filter_polyfill::VecExt as VecDrainFilterExt;
    use leptos_reactive::create_effect;
    use once_cell::unsync::OnceCell;

    use wasm_bindgen::JsCast;
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
                    BuildHasherDefault::<FxHasher>::default(),
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

    // Get items that might have moved
    let from_moved = from.intersection(&to).collect::<FxIndexSet<_>>();
    let to_moved = to.intersection(&from).collect::<FxIndexSet<_>>();

    let ranges = find_ranges(from_moved, to_moved, from, to);
    let mut diffs = Diff {
        removed: removed_cmds.collect(),
        moved: todo!(),
        added: added_cmds.collect(),
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

    ranges
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
#[derive(Debug, PartialEq, Eq)]
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
#[derive(Debug)]
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

    // Resize children if needed
    if cmds.added.len().checked_sub(cmds.removed.len()).is_some() {
        let target_size = children.len()
            + (cmds.added.len() as isize - cmds.removed.len() as isize)
                as usize;

        children.resize_with(target_size, || None);
    }

    // We need to hold a list of items which will be moved, and
    // we can only perform the move after all commands have run, otherwise,
    // we risk overwriting one of the values
    let mut items_to_move = Vec::with_capacity(cmds.moved.len());

    // The order of cmds needs to be:
    // 1. Clear
    // 2. Removed
    // 3. Moved
    // 4. Add
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

    for DiffOpRemove { at } in cmds.removed {
        let item_to_remove = std::mem::take(&mut children[at]).unwrap();

        item_to_remove.prepare_for_move();
    }

    for DiffOpMove {
        from,
        to,
        move_in_dom,
    } in cmds.moved
    {
        let item = std::mem::take(&mut children[from]).unwrap();

        if move_in_dom {
            item.prepare_for_move()
        }

        items_to_move.push((move_in_dom, to, item));
    }

    for DiffOpAdd { at, mode } in cmds.added {
        let item = items[at].take().unwrap();

        let (each_item, _) = cx.run_child_scope(|cx| {
            let child = each_fn(cx, item).into_view(cx);
            EachItem::new(cx, child)
        });

        match mode {
            DiffOpAddMode::Normal => {
                let opening = children.get_next_closest_mounted_sibling(
                    at + 1,
                    closing.to_owned(),
                );

                mount_child(MountKind::Before(&opening), &each_item);
            }
            DiffOpAddMode::Append => {
                mount_child(MountKind::Before(closing), &each_item);
            }
            DiffOpAddMode::_Prepend => todo!(),
        }

        children[at] = Some(each_item);
    }

    for (move_in_dom, to, each_item) in items_to_move {
        if move_in_dom {
            let opening = children
                .get_next_closest_mounted_sibling(to + 1, closing.to_owned());

            mount_child(MountKind::Before(&opening), &each_item);
        }

        children[to] = Some(each_item);
    }

    // Now, remove the holes that might have been left from removing
    // items
    #[allow(unstable_name_collisions)]
    children.drain_filter(|c| c.is_none());
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

    #[test]
    fn single_range() {
        let ranges = find_ranges(
            [1, 2, 3, 4].iter().into_fx_index_set(),
            [1, 2, 3, 4].iter().into_fx_index_set(),
            &[1, 2, 3, 4].into_fx_index_set(),
            &[1, 2, 3, 4].into_fx_index_set(),
        );

        assert_eq!(
            ranges,
            vec![DiffOpMove {
                from: 0,
                to: 0,
                len: 4,
                is_dense: true
            }]
        );
    }

    #[test]
    fn single_range_with_adds() {
        let ranges = find_ranges(
            [1, 2, 3, 4].iter().into_fx_index_set(),
            [1, 2, 3, 4].iter().into_fx_index_set(),
            &[1, 2, 3, 4].into_fx_index_set(),
            &[1, 2, 5, 3, 4].into_fx_index_set(),
        );

        assert_eq!(
            ranges,
            vec![DiffOpMove {
                from: 0,
                to: 0,
                len: 4,
                is_dense: false
            }]
        );
    }

    #[test]
    fn single_range_with_removals() {
        let ranges = find_ranges(
            [1, 2, 3, 4].iter().into_fx_index_set(),
            [1, 2, 3, 4].iter().into_fx_index_set(),
            &[1, 2, 5, 3, 4].into_fx_index_set(),
            &[1, 2, 3, 4].into_fx_index_set(),
        );

        assert_eq!(
            ranges,
            vec![DiffOpMove {
                from: 0,
                to: 0,
                len: 4,
                is_dense: true
            }]
        );
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
}
