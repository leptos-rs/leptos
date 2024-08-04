use super::{
    add_attr::AddAnyAttr, Mountable, Position, PositionState, Render,
    RenderHtml,
};
use crate::{
    html::attribute::Attribute, hydration::Cursor, renderer::Renderer,
    ssr::StreamBuilder,
};
use either_of::Either;
use itertools::Itertools;

/// Retained view state for an `Option`.
pub type OptionState<T, R> =
    Either<<T as Render<R>>::State, <() as Render<R>>::State>;

impl<T, R> Render<R> for Option<T>
where
    T: Render<R>,
    R: Renderer,
{
    type State = OptionState<T, R>;

    fn build(self) -> Self::State {
        match self {
            Some(value) => Either::Left(value),
            None => Either::Right(()),
        }
        .build()
    }

    fn rebuild(self, state: &mut Self::State) {
        match self {
            Some(value) => Either::Left(value),
            None => Either::Right(()),
        }
        .rebuild(state)
    }
}

impl<T, R> AddAnyAttr<R> for Option<T>
where
    T: AddAnyAttr<R>,
    R: Renderer,
{
    type Output<SomeNewAttr: Attribute<R>> =
        Option<<T as AddAnyAttr<R>>::Output<SomeNewAttr>>;

    fn add_any_attr<NewAttr: Attribute<R>>(
        self,
        attr: NewAttr,
    ) -> Self::Output<NewAttr>
    where
        Self::Output<NewAttr>: RenderHtml<R>,
    {
        self.map(|n| n.add_any_attr(attr))
    }
}

impl<T, R> RenderHtml<R> for Option<T>
where
    T: RenderHtml<R>,
    R: Renderer,
{
    type AsyncOutput = Option<T::AsyncOutput>;

    const MIN_LENGTH: usize = T::MIN_LENGTH;

    fn dry_resolve(&mut self) {
        if let Some(inner) = self.as_mut() {
            inner.dry_resolve();
        }
    }

    async fn resolve(self) -> Self::AsyncOutput {
        match self {
            None => None,
            Some(value) => Some(value.resolve().await),
        }
    }

    fn html_len(&self) -> usize {
        match self {
            Some(i) => i.html_len() + 3,
            None => 3,
        }
    }

    fn to_html_with_buf(
        self,
        buf: &mut String,
        position: &mut Position,
        escape: bool,
        mark_branches: bool,
    ) {
        match self {
            Some(value) => Either::Left(value),
            None => Either::Right(()),
        }
        .to_html_with_buf(buf, position, escape, mark_branches)
    }

    fn to_html_async_with_buf<const OUT_OF_ORDER: bool>(
        self,
        buf: &mut StreamBuilder,
        position: &mut Position,
        escape: bool,
        mark_branches: bool,
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
        )
    }

    #[track_caller]
    fn hydrate<const FROM_SERVER: bool>(
        self,
        cursor: &Cursor<R>,
        position: &PositionState,
    ) -> Self::State {
        match self {
            Some(value) => Either::Left(value),
            None => Either::Right(()),
        }
        .hydrate::<FROM_SERVER>(cursor, position)
    }
}

impl<T, R> Render<R> for Vec<T>
where
    T: Render<R>,
    R: Renderer,
{
    type State = VecState<T::State, R>;

    fn build(self) -> Self::State {
        let marker = R::create_placeholder();
        VecState {
            states: self.into_iter().map(T::build).collect(),
            marker,
        }
    }

    fn rebuild(self, state: &mut Self::State) {
        let VecState { states, marker } = state;
        let old = states;
        // this is an unkeyed diff
        if old.is_empty() {
            let mut new = self.build().states;
            for item in new.iter_mut() {
                R::mount_before(item, marker.as_ref());
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
                        T::rebuild(new, old)
                    }
                    itertools::EitherOrBoth::Left(new) => {
                        let mut new_state = new.build();
                        R::mount_before(&mut new_state, marker.as_ref());
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
pub struct VecState<T, R>
where
    T: Mountable<R>,
    R: Renderer,
{
    states: Vec<T>,
    // Vecs keep a placeholder because they have the potential to add additional items,
    // after their own items but before the next neighbor. It is much easier to add an
    // item before a known placeholder than to add it after the last known item, so we
    // just leave a placeholder here unlike zero-or-one iterators (Option, Result, etc.)
    marker: R::Placeholder,
}

impl<T, R> Mountable<R> for VecState<T, R>
where
    T: Mountable<R>,
    R: Renderer,
{
    fn unmount(&mut self) {
        for state in self.states.iter_mut() {
            state.unmount();
        }
        self.marker.unmount();
    }

    fn mount(
        &mut self,
        parent: &<R as Renderer>::Element,
        marker: Option<&<R as Renderer>::Node>,
    ) {
        for state in self.states.iter_mut() {
            state.mount(parent, marker);
        }
        self.marker.mount(parent, marker);
    }

    fn insert_before_this(&self, child: &mut dyn Mountable<R>) -> bool {
        if let Some(first) = self.states.first() {
            first.insert_before_this(child)
        } else {
            false
        }
    }
}

impl<T, R> AddAnyAttr<R> for Vec<T>
where
    T: AddAnyAttr<R>,
    R: Renderer,
{
    type Output<SomeNewAttr: Attribute<R>> =
        Vec<<T as AddAnyAttr<R>>::Output<SomeNewAttr::Cloneable>>;

    fn add_any_attr<NewAttr: Attribute<R>>(
        self,
        attr: NewAttr,
    ) -> Self::Output<NewAttr>
    where
        Self::Output<NewAttr>: RenderHtml<R>,
    {
        let attr = attr.into_cloneable();
        self.into_iter()
            .map(|n| n.add_any_attr(attr.clone()))
            .collect()
    }
}

impl<T, R> RenderHtml<R> for Vec<T>
where
    T: RenderHtml<R>,
    R: Renderer,
{
    type AsyncOutput = Vec<T::AsyncOutput>;

    const MIN_LENGTH: usize = 0;

    fn dry_resolve(&mut self) {
        for inner in self.iter_mut() {
            inner.dry_resolve();
        }
    }

    async fn resolve(self) -> Self::AsyncOutput {
        futures::future::join_all(self.into_iter().map(T::resolve))
            .await
            .into_iter()
            .collect()
    }

    fn html_len(&self) -> usize {
        self.iter().map(|n| n.html_len()).sum::<usize>() + 3
    }

    fn to_html_with_buf(
        self,
        buf: &mut String,
        position: &mut Position,
        escape: bool,
        mark_branches: bool,
    ) {
        let mut children = self.into_iter();
        if let Some(first) = children.next() {
            first.to_html_with_buf(buf, position, escape, mark_branches);
        }
        for child in children {
            child.to_html_with_buf(buf, position, escape, mark_branches);
        }
        buf.push_str("<!>");
    }

    fn to_html_async_with_buf<const OUT_OF_ORDER: bool>(
        self,
        buf: &mut StreamBuilder,
        position: &mut Position,
        escape: bool,
        mark_branches: bool,
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
            );
        }
        for child in children {
            child.to_html_async_with_buf::<OUT_OF_ORDER>(
                buf,
                position,
                escape,
                mark_branches,
            );
        }
        buf.push_sync("<!>");
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        cursor: &Cursor<R>,
        position: &PositionState,
    ) -> Self::State {
        let states = self
            .into_iter()
            .map(|child| child.hydrate::<FROM_SERVER>(cursor, position))
            .collect();

        let marker = cursor.next_placeholder(position);

        VecState { states, marker }
    }
}

/*

pub trait IterView<R: Renderer> {
    type Iterator: Iterator<Item = Self::View>;
    type View: Render<R>;

    fn iter_view(self) -> RenderIter<Self::Iterator, Self::View, R>;
}

impl<I, V, R> IterView<R> for I
where
    I: Iterator<Item = V>,
    V: Render<R>,
    R: Renderer,
{
    type Iterator = I;
    type View = V;

    fn iter_view(self) -> RenderIter<Self::Iterator, Self::View, R> {
        RenderIter {
            inner: self,
            rndr: PhantomData,
        }
    }
}

pub struct RenderIter<I, V, R>
where
    I: Iterator<Item = V>,
    V: Render<R>,
    R: Renderer,
{
    inner: I,
    rndr: PhantomData<R>,
}

impl<I, V, R> Render<R> for RenderIter<I, V, R>
where
    I: Iterator<Item = V>,
    V: Render<R>,
    R: Renderer,
{
    type State = ();

    fn build(self) -> Self::State {
        todo!()
    }

    fn rebuild(self, state: &mut Self::State) {
        todo!()
    }
}

impl<I, V, R> RenderHtml<R> for RenderIter<I, V, R>
where
    I: Iterator<Item = V>,
    V: RenderHtml<R>,
    R: Renderer,

{
    fn to_html(self, buf: &mut String, position: &PositionState) {
        for mut next in self.0.by_ref() {
            next.to_html(buf, position);
        }
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        cursor: &Cursor<R>,
        position: &PositionState,
    ) -> Self::State {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::IterView;
    use crate::view::{Render, RenderHtml};

    #[test]
    fn iter_view_takes_iterator() {
        let strings = vec!["a", "b", "c"];
        let mut iter_view = strings
            .into_iter()
            .map(|n| n.to_ascii_uppercase())
            .iter_view();
        let mut buf = String::new();
        iter_view.to_html(&mut buf, &Default::default());
        assert_eq!(buf, "ABC");
    }
}
*/
