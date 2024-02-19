use super::{
    Mountable, NeverError, Position, PositionState, Render, RenderHtml,
};
use crate::{
    hydration::Cursor,
    renderer::{CastFrom, Renderer},
    ssr::StreamBuilder,
};
use itertools::Itertools;
use std::error::Error;

impl<T, R> Render<R> for Option<T>
where
    T: Render<R>,
    R: Renderer,
{
    type State = OptionState<T::State, R>;
    type FallibleState = OptionState<T::FallibleState, R>;
    type Error = T::Error;

    fn build(self) -> Self::State {
        let placeholder = R::create_placeholder();
        OptionState {
            placeholder,
            state: self.map(T::build),
        }
    }

    fn rebuild(self, state: &mut Self::State) {
        match (&mut state.state, self) {
            // both None: no need to do anything
            (None, None) => {}
            // both Some: need to rebuild child
            (Some(old), Some(new)) => {
                T::rebuild(new, old);
            }
            // Some => None: unmount replace with marker
            (Some(old), None) => {
                old.unmount();
                state.state = None;
            } // None => Some: build
            (None, Some(new)) => {
                let mut new_state = new.build();
                R::mount_before(&mut new_state, state.placeholder.as_ref());
                state.state = Some(new_state);
            }
        }
    }

    fn try_build(self) -> Result<Self::FallibleState, Self::Error> {
        match self {
            None => {
                let placeholder = R::create_placeholder();
                Ok(OptionState {
                    placeholder,
                    state: None,
                })
            }
            Some(inner) => match inner.try_build() {
                Err(e) => return Err(e),
                Ok(inner) => {
                    let placeholder = R::create_placeholder();
                    Ok(OptionState {
                        placeholder,
                        state: Some(inner),
                    })
                }
            },
        }
    }

    fn try_rebuild(
        self,
        state: &mut Self::FallibleState,
    ) -> Result<(), Self::Error> {
        todo!()
    }
}

impl<T, R> RenderHtml<R> for Option<T>
where
    T: RenderHtml<R>,
    R: Renderer,
    R::Node: Clone,
    R::Element: Clone,
{
    const MIN_LENGTH: usize = 0;

    fn to_html_with_buf(self, buf: &mut String, position: &mut Position) {
        if let Some(value) = self {
            value.to_html_with_buf(buf, position);
        }
        // placeholder
        buf.push_str("<!>");
        *position = Position::NextChild;
    }

    fn to_html_async_with_buf<const OUT_OF_ORDER: bool>(
        self,
        buf: &mut StreamBuilder,
        position: &mut Position,
    ) where
        Self: Sized,
    {
        if let Some(value) = self {
            value.to_html_async_with_buf::<OUT_OF_ORDER>(buf, position);
        }
        // placeholder
        buf.push_sync("<!>");
        *position = Position::NextChild;
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        cursor: &Cursor<R>,
        position: &PositionState,
    ) -> Self::State {
        // hydrate the state, if it exists
        let state = self.map(|s| s.hydrate::<FROM_SERVER>(cursor, position));

        // pull the placeholder
        if position.get() == Position::FirstChild {
            cursor.child();
        } else {
            cursor.sibling();
        }
        let placeholder = cursor.current().to_owned();
        let placeholder = R::Placeholder::cast_from(placeholder).unwrap();
        position.set(Position::NextChild);

        OptionState { placeholder, state }
    }
}

/// View state for an optional view.
pub struct OptionState<T, R>
where
    T: Mountable<R>,
    R: Renderer,
{
    /// Marks the location of this view.
    placeholder: R::Placeholder,
    /// The view state.
    state: Option<T>,
}

impl<T, R> Mountable<R> for OptionState<T, R>
where
    T: Mountable<R>,
    R: Renderer,
{
    fn unmount(&mut self) {
        if let Some(ref mut state) = self.state {
            state.unmount();
        }
        R::remove(self.placeholder.as_ref());
    }

    fn mount(&mut self, parent: &R::Element, marker: Option<&R::Node>) {
        if let Some(ref mut state) = self.state {
            state.mount(parent, marker);
        }
        self.placeholder.mount(parent, marker);
    }

    fn insert_before_this(
        &self,
        parent: &R::Element,
        child: &mut dyn Mountable<R>,
    ) -> bool {
        if self
            .state
            .as_ref()
            .map(|n| n.insert_before_this(parent, child))
            == Some(true)
        {
            true
        } else {
            self.placeholder.insert_before_this(parent, child)
        }
    }
}

impl<T, R> Render<R> for Vec<T>
where
    T: Render<R>,
    R: Renderer,
    R::Element: Clone,
    R::Node: Clone,
{
    type State = VecState<T::State, R>;
    type FallibleState = VecState<T::FallibleState, R>;
    type Error = T::Error;

    fn build(self) -> Self::State {
        VecState {
            states: self.into_iter().map(T::build).collect(),
            parent: None,
            marker: None,
        }
    }

    fn rebuild(self, state: &mut Self::State) {
        let VecState {
            states,
            parent,
            marker,
        } = state;
        let old = states;
        // this is an unkeyed diff
        if old.is_empty() {
            let mut new = self.build().states;
            if let Some(parent) = parent {
                for item in new.iter_mut() {
                    item.mount(parent, (*marker).as_ref());
                }
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
                        if let Some(parent) = parent {
                            new_state.mount(parent, (*marker).as_ref());
                        }
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

    fn try_build(self) -> Result<Self::FallibleState, Self::Error> {
        let states = self
            .into_iter()
            .map(T::try_build)
            .collect::<Result<_, _>>()?;
        Ok(VecState {
            states,
            parent: None,
            marker: None,
        })
    }

    fn try_rebuild(
        self,
        state: &mut Self::FallibleState,
    ) -> Result<(), Self::Error> {
        todo!()
    }
}

pub struct VecState<T, R>
where
    T: Mountable<R>,
    R: Renderer,
{
    states: Vec<T>,
    parent: Option<R::Element>,
    marker: Option<R::Node>,
}

impl<T, R> Mountable<R> for VecState<T, R>
where
    T: Mountable<R>,
    R: Renderer,
    R::Element: Clone,
    R::Node: Clone,
{
    fn unmount(&mut self) {
        for state in self.states.iter_mut() {
            state.unmount();
        }
        self.parent = None;
        self.marker = None;
    }

    fn mount(
        &mut self,
        parent: &<R as Renderer>::Element,
        marker: Option<&<R as Renderer>::Node>,
    ) {
        for state in self.states.iter_mut() {
            state.mount(parent, marker);
        }
        self.parent = Some(parent.clone());
        self.marker = marker.cloned();
    }

    fn insert_before_this(
        &self,
        parent: &<R as Renderer>::Element,
        child: &mut dyn Mountable<R>,
    ) -> bool {
        if let Some(first) = self.states.get(0) {
            first.insert_before_this(parent, child)
        } else {
            false
        }
    }
}

impl<T, R> RenderHtml<R> for Vec<T>
where
    T: RenderHtml<R>,
    R: Renderer,
    R::Node: Clone,
    R::Element: Clone,
{
    const MIN_LENGTH: usize = 0;

    fn to_html_with_buf(self, buf: &mut String, position: &mut Position) {
        let mut children = self.into_iter();
        if let Some(first) = children.next() {
            first.to_html_with_buf(buf, position);
        }
        for child in children {
            child.to_html_with_buf(buf, position);
        }
    }

    fn to_html_async_with_buf<const OUT_OF_ORDER: bool>(
        self,
        buf: &mut StreamBuilder,
        position: &mut Position,
    ) where
        Self: Sized,
    {
        let mut children = self.into_iter();
        if let Some(first) = children.next() {
            first.to_html_async_with_buf::<OUT_OF_ORDER>(buf, position);
        }
        for child in children {
            child.to_html_async_with_buf::<OUT_OF_ORDER>(buf, position);
        }
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        cursor: &Cursor<R>,
        position: &PositionState,
    ) -> Self::State {
        // TODO does this make sense for hydration from template?
        VecState {
            states: self
                .into_iter()
                .map(|child| child.hydrate::<FROM_SERVER>(cursor, position))
                .collect(),
            parent: None,
            marker: None,
        }
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
    R::Node: Clone,
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
