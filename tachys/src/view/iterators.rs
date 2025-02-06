use super::{
    add_attr::AddAnyAttr, any_view::ExtraAttrsMut,
    batch_resolve_items_with_extra_attrs, Mountable, Position, PositionState,
    Render, RenderHtml,
};
use crate::{
    html::attribute::{any_attribute::AnyAttribute, Attribute},
    hydration::Cursor,
    renderer::Rndr,
    ssr::StreamBuilder,
};
use either_of::Either;
use itertools::Itertools;

/// Retained view state for an `Option`.
pub type OptionState<T> = Either<<T as Render>::State, <() as Render>::State>;

impl<T> Render for Option<T>
where
    T: Render,
{
    type State = OptionState<T>;

    fn build(self, extra_attrs: Option<Vec<AnyAttribute>>) -> Self::State {
        match self {
            Some(value) => Either::Left(value),
            None => Either::Right(()),
        }
        .build(extra_attrs)
    }

    fn rebuild(
        self,
        state: &mut Self::State,
        extra_attrs: Option<Vec<AnyAttribute>>,
    ) {
        match self {
            Some(value) => Either::Left(value),
            None => Either::Right(()),
        }
        .rebuild(state, extra_attrs)
    }
}

impl<T> AddAnyAttr for Option<T>
where
    T: AddAnyAttr,
{
    type Output<SomeNewAttr: Attribute> =
        Option<<T as AddAnyAttr>::Output<SomeNewAttr>>;

    fn add_any_attr<NewAttr: Attribute>(
        self,
        attr: NewAttr,
    ) -> Self::Output<NewAttr>
    where
        Self::Output<NewAttr>: RenderHtml,
    {
        self.map(|n| n.add_any_attr(attr))
    }
}

impl<T> RenderHtml for Option<T>
where
    T: RenderHtml,
{
    type AsyncOutput = Option<T::AsyncOutput>;
    type Owned = Option<T::Owned>;

    const MIN_LENGTH: usize = T::MIN_LENGTH;

    fn dry_resolve(&mut self, extra_attrs: ExtraAttrsMut<'_>) {
        if let Some(inner) = self.as_mut() {
            inner.dry_resolve(extra_attrs);
        }
    }

    async fn resolve(
        self,
        extra_attrs: ExtraAttrsMut<'_>,
    ) -> Self::AsyncOutput {
        match self {
            None => None,
            Some(value) => Some(value.resolve(extra_attrs).await),
        }
    }

    fn html_len(&self, extra_attrs: Option<Vec<&AnyAttribute>>) -> usize {
        match self {
            Some(i) => i.html_len(extra_attrs) + 3,
            None => 3,
        }
    }

    fn to_html_with_buf(
        self,
        buf: &mut String,
        position: &mut Position,
        escape: bool,
        mark_branches: bool,
        extra_attrs: Option<Vec<AnyAttribute>>,
    ) {
        match self {
            Some(value) => Either::Left(value),
            None => Either::Right(()),
        }
        .to_html_with_buf(
            buf,
            position,
            escape,
            mark_branches,
            extra_attrs,
        )
    }

    fn to_html_async_with_buf<const OUT_OF_ORDER: bool>(
        self,
        buf: &mut StreamBuilder,
        position: &mut Position,
        escape: bool,
        mark_branches: bool,
        extra_attrs: Option<Vec<AnyAttribute>>,
    ) where
        Self: Sized,
    {
        match self {
            Some(value) => Either::Left(value),
            None => Either::Right(()),
        }
        .to_html_async_with_buf::<OUT_OF_ORDER>(
            buf,
            position,
            escape,
            mark_branches,
            extra_attrs,
        )
    }

    #[track_caller]
    fn hydrate<const FROM_SERVER: bool>(
        self,
        cursor: &Cursor,
        position: &PositionState,
        extra_attrs: Option<Vec<AnyAttribute>>,
    ) -> Self::State {
        match self {
            Some(value) => Either::Left(value),
            None => Either::Right(()),
        }
        .hydrate::<FROM_SERVER>(cursor, position, extra_attrs)
    }

    fn into_owned(self) -> Self::Owned {
        self.map(RenderHtml::into_owned)
    }
}

impl<T> Render for Vec<T>
where
    T: Render,
{
    type State = VecState<T::State>;

    fn build(self, extra_attrs: Option<Vec<AnyAttribute>>) -> Self::State {
        let marker = Rndr::create_placeholder();
        VecState {
            states: self
                .into_iter()
                .map(|val| T::build(val, extra_attrs.clone()))
                .collect(),
            marker,
        }
    }

    fn rebuild(
        self,
        state: &mut Self::State,
        extra_attrs: Option<Vec<AnyAttribute>>,
    ) {
        let VecState { states, marker } = state;
        let old = states;
        // this is an unkeyed diff
        if old.is_empty() {
            let mut new = self.build(extra_attrs).states;
            for item in new.iter_mut() {
                Rndr::mount_before(item, marker.as_ref());
            }
            *old = new;
        } else if self.is_empty() {
            // TODO fast path for clearing
            for item in old.iter_mut() {
                item.unmount();
            }
            old.clear();
        } else {
            let mut adds = vec![];
            let mut removes_at_end = 0;
            for item in self.into_iter().zip_longest(old.iter_mut()) {
                match item {
                    itertools::EitherOrBoth::Both(new, old) => {
                        T::rebuild(new, old, extra_attrs.clone())
                    }
                    itertools::EitherOrBoth::Left(new) => {
                        let mut new_state = new.build(extra_attrs.clone());
                        Rndr::mount_before(&mut new_state, marker.as_ref());
                        adds.push(new_state);
                    }
                    itertools::EitherOrBoth::Right(old) => {
                        removes_at_end += 1;
                        old.unmount()
                    }
                }
            }
            old.truncate(old.len() - removes_at_end);
            old.append(&mut adds);
        }
    }
}

/// Retained view state for a `Vec<_>`.
pub struct VecState<T>
where
    T: Mountable,
{
    states: Vec<T>,
    // Vecs keep a placeholder because they have the potential to add additional items,
    // after their own items but before the next neighbor. It is much easier to add an
    // item before a known placeholder than to add it after the last known item, so we
    // just leave a placeholder here unlike zero-or-one iterators (Option, Result, etc.)
    marker: crate::renderer::types::Placeholder,
}

impl<T> Mountable for VecState<T>
where
    T: Mountable,
{
    fn unmount(&mut self) {
        for state in self.states.iter_mut() {
            state.unmount();
        }
        self.marker.unmount();
    }

    fn mount(
        &mut self,
        parent: &crate::renderer::types::Element,
        marker: Option<&crate::renderer::types::Node>,
    ) {
        for state in self.states.iter_mut() {
            state.mount(parent, marker);
        }
        self.marker.mount(parent, marker);
    }

    fn insert_before_this(&self, child: &mut dyn Mountable) -> bool {
        if let Some(first) = self.states.first() {
            first.insert_before_this(child)
        } else {
            self.marker.insert_before_this(child)
        }
    }
}

impl<T> AddAnyAttr for Vec<T>
where
    T: AddAnyAttr,
{
    type Output<SomeNewAttr: Attribute> =
        Vec<<T as AddAnyAttr>::Output<SomeNewAttr::Cloneable>>;

    fn add_any_attr<NewAttr: Attribute>(
        self,
        attr: NewAttr,
    ) -> Self::Output<NewAttr>
    where
        Self::Output<NewAttr>: RenderHtml,
    {
        let attr = attr.into_cloneable();
        self.into_iter()
            .map(|n| n.add_any_attr(attr.clone()))
            .collect()
    }
}

impl<T> RenderHtml for Vec<T>
where
    T: RenderHtml,
{
    type AsyncOutput = Vec<T::AsyncOutput>;
    type Owned = Vec<T::Owned>;

    const MIN_LENGTH: usize = 0;

    fn dry_resolve(&mut self, mut extra_attrs: ExtraAttrsMut<'_>) {
        for inner in self.iter_mut() {
            inner.dry_resolve(extra_attrs.as_deref_mut());
        }
    }

    async fn resolve(
        self,
        extra_attrs: ExtraAttrsMut<'_>,
    ) -> Self::AsyncOutput {
        batch_resolve_items_with_extra_attrs(self, extra_attrs)
            .await
            .into_iter()
            .collect()
    }

    fn html_len(&self, extra_attrs: Option<Vec<&AnyAttribute>>) -> usize {
        self.iter()
            .map(|n| n.html_len(extra_attrs.clone()))
            .sum::<usize>()
            + 3
    }

    fn to_html_with_buf(
        self,
        buf: &mut String,
        position: &mut Position,
        escape: bool,
        mark_branches: bool,
        extra_attrs: Option<Vec<AnyAttribute>>,
    ) {
        let mut children = self.into_iter();
        if let Some(first) = children.next() {
            first.to_html_with_buf(
                buf,
                position,
                escape,
                mark_branches,
                extra_attrs.clone(),
            );
        }
        for child in children {
            child.to_html_with_buf(
                buf,
                position,
                escape,
                mark_branches,
                extra_attrs.clone(),
            );
        }
        buf.push_str("<!>");
    }

    fn to_html_async_with_buf<const OUT_OF_ORDER: bool>(
        self,
        buf: &mut StreamBuilder,
        position: &mut Position,
        escape: bool,
        mark_branches: bool,
        extra_attrs: Option<Vec<AnyAttribute>>,
    ) where
        Self: Sized,
    {
        let mut children = self.into_iter();
        if let Some(first) = children.next() {
            first.to_html_async_with_buf::<OUT_OF_ORDER>(
                buf,
                position,
                escape,
                mark_branches,
                extra_attrs.clone(),
            );
        }
        for child in children {
            child.to_html_async_with_buf::<OUT_OF_ORDER>(
                buf,
                position,
                escape,
                mark_branches,
                extra_attrs.clone(),
            );
        }
        buf.push_sync("<!>");
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        cursor: &Cursor,
        position: &PositionState,
        extra_attrs: Option<Vec<AnyAttribute>>,
    ) -> Self::State {
        let states = self
            .into_iter()
            .map(|child| {
                child.hydrate::<FROM_SERVER>(
                    cursor,
                    position,
                    extra_attrs.clone(),
                )
            })
            .collect();

        let marker = cursor.next_placeholder(position);

        VecState { states, marker }
    }

    fn into_owned(self) -> Self::Owned {
        self.into_iter().map(RenderHtml::into_owned).collect()
    }
}

impl<T, const N: usize> Render for [T; N]
where
    T: Render,
{
    type State = ArrayState<T::State, N>;

    fn build(self, extra_attrs: Option<Vec<AnyAttribute>>) -> Self::State {
        Self::State {
            states: self.map(|val| T::build(val, extra_attrs.clone())),
        }
    }

    fn rebuild(
        self,
        state: &mut Self::State,
        extra_attrs: Option<Vec<AnyAttribute>>,
    ) {
        let Self::State { states } = state;
        let old = states;
        // this is an unkeyed diff
        self.into_iter()
            .zip(old.iter_mut())
            .for_each(|(new, old)| T::rebuild(new, old, extra_attrs.clone()));
    }
}

/// Retained view state for a `Vec<_>`.
pub struct ArrayState<T, const N: usize>
where
    T: Mountable,
{
    states: [T; N],
}

impl<T, const N: usize> Mountable for ArrayState<T, N>
where
    T: Mountable,
{
    fn unmount(&mut self) {
        self.states.iter_mut().for_each(Mountable::unmount);
    }

    fn mount(
        &mut self,
        parent: &crate::renderer::types::Element,
        marker: Option<&crate::renderer::types::Node>,
    ) {
        for state in self.states.iter_mut() {
            state.mount(parent, marker);
        }
    }

    fn insert_before_this(&self, child: &mut dyn Mountable) -> bool {
        if let Some(first) = self.states.first() {
            first.insert_before_this(child)
        } else {
            false
        }
    }
}
impl<T, const N: usize> AddAnyAttr for [T; N]
where
    T: AddAnyAttr,
{
    type Output<SomeNewAttr: Attribute> =
        [<T as AddAnyAttr>::Output<SomeNewAttr::Cloneable>; N];

    fn add_any_attr<NewAttr: Attribute>(
        self,
        attr: NewAttr,
    ) -> Self::Output<NewAttr>
    where
        Self::Output<NewAttr>: RenderHtml,
    {
        let attr = attr.into_cloneable();
        self.map(|n| n.add_any_attr(attr.clone()))
    }
}

impl<T, const N: usize> RenderHtml for [T; N]
where
    T: RenderHtml,
{
    type AsyncOutput = [T::AsyncOutput; N];
    type Owned = Vec<T::Owned>;

    const MIN_LENGTH: usize = 0;

    fn dry_resolve(&mut self, mut extra_attrs: ExtraAttrsMut<'_>) {
        for inner in self.iter_mut() {
            inner.dry_resolve(extra_attrs.as_deref_mut());
        }
    }

    async fn resolve(
        self,
        extra_attrs: ExtraAttrsMut<'_>,
    ) -> Self::AsyncOutput {
        batch_resolve_items_with_extra_attrs(self, extra_attrs)
            .await
            .into_iter()
            .collect::<Vec<_>>()
            .try_into()
            .unwrap_or_else(|_| unreachable!())
    }

    fn html_len(&self, extra_attrs: Option<Vec<&AnyAttribute>>) -> usize {
        self.iter()
            .map(|val| RenderHtml::html_len(val, extra_attrs.clone()))
            .sum::<usize>()
    }

    fn to_html_with_buf(
        self,
        buf: &mut String,
        position: &mut Position,
        escape: bool,
        mark_branches: bool,
        extra_attrs: Option<Vec<AnyAttribute>>,
    ) {
        for child in self.into_iter() {
            child.to_html_with_buf(
                buf,
                position,
                escape,
                mark_branches,
                extra_attrs.clone(),
            );
        }
    }

    fn to_html_async_with_buf<const OUT_OF_ORDER: bool>(
        self,
        buf: &mut StreamBuilder,
        position: &mut Position,
        escape: bool,
        mark_branches: bool,
        extra_attrs: Option<Vec<AnyAttribute>>,
    ) where
        Self: Sized,
    {
        for child in self.into_iter() {
            child.to_html_async_with_buf::<OUT_OF_ORDER>(
                buf,
                position,
                escape,
                mark_branches,
                extra_attrs.clone(),
            );
        }
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        cursor: &Cursor,
        position: &PositionState,
        extra_attrs: Option<Vec<AnyAttribute>>,
    ) -> Self::State {
        let states = self.map(|child| {
            child.hydrate::<FROM_SERVER>(cursor, position, extra_attrs.clone())
        });
        ArrayState { states }
    }

    fn into_owned(self) -> Self::Owned {
        self.into_iter()
            .map(RenderHtml::into_owned)
            .collect::<Vec<_>>()
    }
}
