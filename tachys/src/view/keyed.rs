use super::{
    add_attr::AddAnyAttr, Mountable, Position, PositionState, Render,
    RenderHtml,
};
use crate::{
    html::attribute::Attribute,
    hydration::Cursor,
    renderer::{CastFrom, Rndr},
    ssr::StreamBuilder,
};
use drain_filter_polyfill::VecExt as VecDrainFilterExt;
use indexmap::IndexSet;
use rustc_hash::FxHasher;
use std::hash::{BuildHasherDefault, Hash};

type FxIndexSet<T> = IndexSet<T, BuildHasherDefault<FxHasher>>;

/// Creates a keyed list of views.
pub fn keyed<T, I, K, KF, VF, VFS, V>(
    items: I,
    key_fn: KF,
    view_fn: VF,
) -> Keyed<T, I, K, KF, VF, VFS, V>
where
    I: IntoIterator<Item = T>,
    K: Eq + Hash + 'static,
    KF: Fn(&T) -> K,
    V: Render,
    VF: Fn(usize, T) -> (VFS, V),
    VFS: Fn(usize),
{
    Keyed {
        items,
        key_fn,
        view_fn,
    }
}

/// A keyed list of views.
pub struct Keyed<T, I, K, KF, VF, VFS, V>
where
    I: IntoIterator<Item = T>,
    K: Eq + Hash + 'static,
    KF: Fn(&T) -> K,
    VF: Fn(usize, T) -> (VFS, V),
    VFS: Fn(usize),
{
    items: I,
    key_fn: KF,
    view_fn: VF,
}

/// Retained view state for a keyed list.
pub struct KeyedState<K, VFS, V>
where
    K: Eq + Hash + 'static,
    VFS: Fn(usize),
    V: Render,
{
    parent: Option<crate::renderer::types::Element>,
    marker: crate::renderer::types::Placeholder,
    hashed_items: IndexSet<K, BuildHasherDefault<FxHasher>>,
    rendered_items: Vec<Option<(VFS, V::State)>>,
}

impl<T, I, K, KF, VF, VFS, V> Render for Keyed<T, I, K, KF, VF, VFS, V>
where
    I: IntoIterator<Item = T>,
    K: Eq + Hash + 'static,
    KF: Fn(&T) -> K,
    V: Render,
    VF: Fn(usize, T) -> (VFS, V),
    VFS: Fn(usize),
{
    type State = KeyedState<K, VFS, V>;
    // TODO fallible state and try_build()/try_rebuild() here

    fn build(self) -> Self::State {
        let items = self.items.into_iter();
        let (capacity, _) = items.size_hint();
        let mut hashed_items =
            FxIndexSet::with_capacity_and_hasher(capacity, Default::default());
        let mut rendered_items = Vec::new();
        for (index, item) in items.enumerate() {
            hashed_items.insert((self.key_fn)(&item));
            let (set_index, view) = (self.view_fn)(index, item);
            rendered_items.push(Some((set_index, view.build())));
        }
        KeyedState {
            parent: None,
            marker: Rndr::create_placeholder(),
            hashed_items,
            rendered_items,
        }
    }

    fn rebuild(self, state: &mut Self::State) {
        let KeyedState {
            parent,
            marker,
            hashed_items,
            ref mut rendered_items,
        } = state;
        let new_items = self.items.into_iter();
        let (capacity, _) = new_items.size_hint();
        let mut new_hashed_items =
            FxIndexSet::with_capacity_and_hasher(capacity, Default::default());

        let mut items = Vec::new();
        for item in new_items {
            new_hashed_items.insert((self.key_fn)(&item));
            items.push(Some(item));
        }

        let cmds = diff(hashed_items, &new_hashed_items);

        apply_diff(
            parent
                .as_ref()
                .expect("Keyed list rebuilt before being mounted."),
            marker,
            cmds,
            rendered_items,
            &self.view_fn,
            items,
        );

        *hashed_items = new_hashed_items;
    }
}

impl<T, I, K, KF, VF, VFS, V> AddAnyAttr for Keyed<T, I, K, KF, VF, VFS, V>
where
    I: IntoIterator<Item = T> + Send,
    K: Eq + Hash + 'static,
    KF: Fn(&T) -> K + Send,
    V: RenderHtml,
    V: 'static,
    VF: Fn(usize, T) -> (VFS, V) + Send + 'static,
    VFS: Fn(usize) + 'static,
    T: 'static,
{
    type Output<SomeNewAttr: Attribute> = Keyed<
        T,
        I,
        K,
        KF,
        Box<
            dyn Fn(
                    usize,
                    T,
                ) -> (
                    VFS,
                    <V as AddAnyAttr>::Output<SomeNewAttr::CloneableOwned>,
                ) + Send,
        >,
        VFS,
        V::Output<SomeNewAttr::CloneableOwned>,
    >;

    fn add_any_attr<NewAttr: Attribute>(
        self,
        attr: NewAttr,
    ) -> Self::Output<NewAttr>
    where
        Self::Output<NewAttr>: RenderHtml,
    {
        let Keyed {
            items,
            key_fn,
            view_fn,
        } = self;
        let attr = attr.into_cloneable_owned();
        Keyed {
            items,
            key_fn,
            view_fn: Box::new(move |index, item| {
                let (index, view) = view_fn(index, item);
                (index, view.add_any_attr(attr.clone()))
            }),
        }
    }
}

impl<T, I, K, KF, VF, VFS, V> RenderHtml for Keyed<T, I, K, KF, VF, VFS, V>
where
    I: IntoIterator<Item = T> + Send,
    K: Eq + Hash + 'static,
    KF: Fn(&T) -> K + Send,
    V: RenderHtml + 'static,
    VF: Fn(usize, T) -> (VFS, V) + Send + 'static,
    VFS: Fn(usize) + 'static,
    T: 'static,
{
    type AsyncOutput = Vec<V::AsyncOutput>; // TODO

    const MIN_LENGTH: usize = 0;

    fn dry_resolve(&mut self) {
        // TODO...
    }

    async fn resolve(self) -> Self::AsyncOutput {
        futures::future::join_all(self.items.into_iter().enumerate().map(
            |(index, item)| {
                let (_, view) = (self.view_fn)(index, item);
                view.resolve()
            },
        ))
        .await
        .into_iter()
        .collect::<Vec<_>>()
    }

    fn to_html_with_buf(
        self,
        buf: &mut String,
        position: &mut Position,
        escape: bool,
        mark_branches: bool,
    ) {
        for (index, item) in self.items.into_iter().enumerate() {
            let (_, item) = (self.view_fn)(index, item);
            item.to_html_with_buf(buf, position, escape, mark_branches);
            *position = Position::NextChild;
        }
        buf.push_str("<!>");
    }

    fn to_html_async_with_buf<const OUT_OF_ORDER: bool>(
        self,
        buf: &mut StreamBuilder,
        position: &mut Position,
        escape: bool,
        mark_branches: bool,
    ) {
        for (index, item) in self.items.into_iter().enumerate() {
            let (_, item) = (self.view_fn)(index, item);
            item.to_html_async_with_buf::<OUT_OF_ORDER>(
                buf,
                position,
                escape,
                mark_branches,
            );
            *position = Position::NextChild;
        }
        buf.push_sync("<!>");
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        cursor: &Cursor,
        position: &PositionState,
    ) -> Self::State {
        // get parent and position
        let current = cursor.current();
        let parent = if position.get() == Position::FirstChild {
            current
        } else {
            Rndr::get_parent(&current)
                .expect("first child of keyed list has no parent")
        };
        let parent = crate::renderer::types::Element::cast_from(parent)
            .expect("parent of keyed list should be an element");

        // build list
        let items = self.items.into_iter();
        let (capacity, _) = items.size_hint();
        let mut hashed_items =
            FxIndexSet::with_capacity_and_hasher(capacity, Default::default());
        let mut rendered_items = Vec::new();
        for (index, item) in items.enumerate() {
            hashed_items.insert((self.key_fn)(&item));
            let (set_index, view) = (self.view_fn)(index, item);
            let item = view.hydrate::<FROM_SERVER>(cursor, position);
            rendered_items.push(Some((set_index, item)));
        }
        let marker = cursor.next_placeholder(position);
        KeyedState {
            parent: Some(parent),
            marker,
            hashed_items,
            rendered_items,
        }
    }
}

impl<K, VFS, V> Mountable for KeyedState<K, VFS, V>
where
    K: Eq + Hash + 'static,
    VFS: Fn(usize),
    V: Render,
{
    fn mount(
        &mut self,
        parent: &crate::renderer::types::Element,
        marker: Option<&crate::renderer::types::Node>,
    ) {
        self.parent = Some(parent.clone());
        for (_, item) in self.rendered_items.iter_mut().flatten() {
            item.mount(parent, marker);
        }
        self.marker.mount(parent, marker);
    }

    fn unmount(&mut self) {
        for (_, item) in self.rendered_items.iter_mut().flatten() {
            item.unmount();
        }
        self.marker.unmount();
    }

    fn insert_before_this(&self, child: &mut dyn Mountable) -> bool {
        self.rendered_items
            .first()
            .map(|item| {
                if let Some((_, item)) = item {
                    item.insert_before_this(child)
                } else {
                    false
                }
            })
            .unwrap_or_else(|| self.marker.insert_before_this(child))
    }
}

trait VecExt<T> {
    fn get_next_closest_mounted_sibling(
        &self,
        start_at: usize,
    ) -> Option<&Option<T>>;
}

impl<T> VecExt<T> for Vec<Option<T>> {
    fn get_next_closest_mounted_sibling(
        &self,
        start_at: usize,
    ) -> Option<&Option<T>> {
        self[start_at..].iter().find(|s| s.is_some())
    }
}

/// Calculates the operations needed to get from `from` to `to`.
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DiffOpAddMode {
    Normal,
    Append,
}

impl Default for DiffOpAddMode {
    fn default() -> Self {
        Self::Normal
    }
}

fn apply_diff<T, VFS, V>(
    parent: &crate::renderer::types::Element,
    marker: &crate::renderer::types::Placeholder,
    diff: Diff,
    children: &mut Vec<Option<(VFS, V::State)>>,
    view_fn: impl Fn(usize, T) -> (VFS, V),
    mut items: Vec<Option<T>>,
) where
    VFS: Fn(usize),
    V: Render,
{
    // The order of cmds needs to be:
    // 1. Clear
    // 2. Removals
    // 3. Move out
    // 4. Resize
    // 5. Move in
    // 6. Additions
    // 7. Removes holes
    if diff.clear {
        for (_, mut child) in children.drain(0..).flatten() {
            child.unmount();
        }

        if diff.added.is_empty() {
            return;
        }
    }

    for DiffOpRemove { at } in &diff.removed {
        let (_, mut item_to_remove) = children[*at].take().unwrap();

        item_to_remove.unmount();
    }

    let (move_cmds, add_cmds) = unpack_moves(&diff);

    let mut moved_children = move_cmds
        .iter()
        .map(|move_| children[move_.from].take())
        .collect::<Vec<_>>();

    children.resize_with(children.len() + diff.added.len(), || None);

    for (i, DiffOpMove { to, .. }) in move_cmds
        .iter()
        .enumerate()
        .filter(|(_, move_)| !move_.move_in_dom)
    {
        children[*to] = moved_children[i]
            .take()
            .inspect(|(set_index, _)| set_index(*to));
    }

    for (i, DiffOpMove { to, .. }) in move_cmds
        .into_iter()
        .enumerate()
        .filter(|(_, move_)| move_.move_in_dom)
    {
        let (set_index, mut each_item) = moved_children[i].take().unwrap();

        if let Some(Some((_, state))) =
            children.get_next_closest_mounted_sibling(to)
        {
            state.insert_before_this_or_marker(
                parent,
                &mut each_item,
                Some(marker.as_ref()),
            )
        } else {
            each_item.mount(parent, Some(marker.as_ref()));
        }

        set_index(to);
        children[to] = Some((set_index, each_item));
    }

    for DiffOpAdd { at, mode } in add_cmds {
        let item = items[at].take().unwrap();
        let (set_index, item) = view_fn(at, item);
        let mut item = item.build();

        match mode {
            DiffOpAddMode::Normal => {
                if let Some(Some((_, state))) =
                    children.get_next_closest_mounted_sibling(at)
                {
                    state.insert_before_this_or_marker(
                        parent,
                        &mut item,
                        Some(marker.as_ref()),
                    )
                } else {
                    item.mount(parent, Some(marker.as_ref()));
                }
            }
            DiffOpAddMode::Append => {
                item.mount(parent, Some(marker.as_ref()));
            }
        }

        children[at] = Some((set_index, item));
    }

    #[allow(unstable_name_collisions)]
    children.drain_filter(|c| c.is_none());
}

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
/*
#[cfg(test)]
mod tests {
    use crate::{
        html::element::{li, ul, HtmlElement, Li},
        renderer::mock_dom::MockDom,
        view::{keyed::keyed, Render},
    };

    fn item(key: usize) -> HtmlElement<Li, (), String, MockDom> {
        li((), key.to_string())
    }

    #[test]
    fn keyed_creates_list() {
        let el = ul((), keyed(1..=3, |k| *k, item));
        let el_state = el.build();
        assert_eq!(
            el_state.el.to_debug_html(),
            "<ul><li>1</li><li>2</li><li>3</li></ul>"
        );
    }

    #[test]
    fn adding_items_updates_list() {
        let el = ul((), keyed(1..=3, |k| *k, item));
        let mut el_state = el.build();
        let el = ul((), keyed(1..=5, |k| *k, item));
        el.rebuild(&mut el_state);
        assert_eq!(
            el_state.el.to_debug_html(),
            "<ul><li>1</li><li>2</li><li>3</li><li>4</li><li>5</li></ul>"
        );
    }

    #[test]
    fn removing_items_updates_list() {
        let el = ul((), keyed(1..=3, |k| *k, item));
        let mut el_state = el.build();
        let el = ul((), keyed(1..=2, |k| *k, item));
        el.rebuild(&mut el_state);
        assert_eq!(
            el_state.el.to_debug_html(),
            "<ul><li>1</li><li>2</li></ul>"
        );
    }

    #[test]
    fn swapping_items_updates_list() {
        let el = ul((), keyed([1, 2, 3, 4, 5], |k| *k, item));
        let mut el_state = el.build();
        let el = ul((), keyed([1, 4, 3, 2, 5], |k| *k, item));
        el.rebuild(&mut el_state);
        assert_eq!(
            el_state.el.to_debug_html(),
            "<ul><li>1</li><li>4</li><li>3</li><li>2</li><li>5</li></ul>"
        );
    }

    #[test]
    fn swapping_and_removing_orders_correctly() {
        let el = ul((), keyed([1, 2, 3, 4, 5], |k| *k, item));
        let mut el_state = el.build();
        let el = ul((), keyed([1, 4, 3, 5], |k| *k, item));
        el.rebuild(&mut el_state);
        assert_eq!(
            el_state.el.to_debug_html(),
            "<ul><li>1</li><li>4</li><li>3</li><li>5</li></ul>"
        );
    }

    #[test]
    fn arbitrarily_hard_adjustment() {
        let el = ul((), keyed([1, 2, 3, 4, 5], |k| *k, item));
        let mut el_state = el.build();
        let el = ul((), keyed([2, 4, 3], |k| *k, item));
        el.rebuild(&mut el_state);
        assert_eq!(
            el_state.el.to_debug_html(),
            "<ul><li>2</li><li>4</li><li>3</li></ul>"
        );
    }

    #[test]
    fn a_series_of_moves() {
        let el = ul((), keyed([1, 2, 3, 4, 5], |k| *k, item));
        let mut el_state = el.build();
        let el = ul((), keyed([2, 4, 3], |k| *k, item));
        el.rebuild(&mut el_state);
        let el = ul((), keyed([1, 7, 5, 11, 13, 17], |k| *k, item));
        el.rebuild(&mut el_state);
        let el = ul((), keyed([2, 6, 8, 7, 13], |k| *k, item));
        el.rebuild(&mut el_state);
        let el = ul((), keyed([13, 4, 5, 3], |k| *k, item));
        el.rebuild(&mut el_state);
        let el = ul((), keyed([1, 2, 3, 4], |k| *k, item));
        el.rebuild(&mut el_state);
        assert_eq!(
            el_state.el.to_debug_html(),
            "<ul><li>1</li><li>2</li><li>3</li><li>4</li></ul>"
        );
    }

    #[test]
    fn clearing_works() {
        let el = ul((), keyed([1, 2, 3, 4, 5], |k| *k, item));
        let mut el_state = el.build();
        let el = ul((), keyed([], |k| *k, item));
        el.rebuild(&mut el_state);
        assert_eq!(el_state.el.to_debug_html(), "<ul></ul>");
    }
}
*/
