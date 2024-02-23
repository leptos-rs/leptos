use crate::{
    hydration::Cursor,
    prelude::Mountable,
    renderer::Renderer,
    ssr::StreamBuilder,
    view::{Position, PositionState, Render, RenderHtml},
};
use reactive_graph::owner::Owner;
use std::marker::PhantomData;

#[derive(Debug, Clone)]
pub struct OwnedView<T, R> {
    owner: Owner,
    view: T,
    rndr: PhantomData<R>,
}

impl<T, R> OwnedView<T, R> {
    /// Wraps a view with the current owner.
    pub fn new(view: T) -> Self {
        let owner = Owner::current().expect("no reactive owner");
        Self {
            owner,
            view,
            rndr: PhantomData,
        }
    }

    /// Wraps a view with the given owner.
    pub fn new_with_owner(view: T, owner: Owner) -> Self {
        Self {
            owner,
            view,
            rndr: PhantomData,
        }
    }
}

#[derive(Debug, Clone)]
pub struct OwnedViewState<T, R>
where
    T: Mountable<R>,
    R: Renderer,
{
    owner: Owner,
    state: T,
    rndr: PhantomData<R>,
}

impl<T, R> OwnedViewState<T, R>
where
    T: Mountable<R>,
    R: Renderer,
{
    /// Wraps a state with the given owner.
    fn new(state: T, owner: Owner) -> Self {
        Self {
            owner,
            state,
            rndr: PhantomData,
        }
    }
}

impl<T, R> Render<R> for OwnedView<T, R>
where
    T: Render<R>,
    R: Renderer,
{
    type State = OwnedViewState<T::State, R>;
    type FallibleState = OwnedViewState<T::FallibleState, R>;

    fn build(self) -> Self::State {
        let state = self.owner.with(|| self.view.build());
        OwnedViewState::new(state, self.owner)
    }

    fn rebuild(self, state: &mut Self::State) {
        let OwnedView { owner, view, .. } = self;
        owner.with(|| view.rebuild(&mut state.state));
        state.owner = owner;
    }

    fn try_build(self) -> crate::error::Result<Self::FallibleState> {
        todo!()
    }

    fn try_rebuild(
        self,
        _state: &mut Self::FallibleState,
    ) -> crate::error::Result<()> {
        todo!()
    }
}

impl<T, R> RenderHtml<R> for OwnedView<T, R>
where
    T: RenderHtml<R>,
    R: Renderer,
{
    const MIN_LENGTH: usize = T::MIN_LENGTH;

    fn to_html_with_buf(
        self,
        buf: &mut String,
        position: &mut crate::view::Position,
    ) {
        self.owner
            .with(|| self.view.to_html_with_buf(buf, position));
    }

    fn to_html_async_with_buf<const OUT_OF_ORDER: bool>(
        self,
        buf: &mut StreamBuilder,
        position: &mut Position,
    ) where
        Self: Sized,
    {
        self.owner.with(|| {
            self.view
                .to_html_async_with_buf::<OUT_OF_ORDER>(buf, position)
        });
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        cursor: &Cursor<R>,
        position: &PositionState,
    ) -> Self::State {
        let state = self
            .owner
            .with(|| self.view.hydrate::<FROM_SERVER>(cursor, position));
        OwnedViewState::new(state, self.owner)
    }
}

impl<T, R> Mountable<R> for OwnedViewState<T, R>
where
    T: Mountable<R>,
    R: Renderer,
{
    fn unmount(&mut self) {
        self.state.unmount();
    }

    fn mount(
        &mut self,
        parent: &<R as Renderer>::Element,
        marker: Option<&<R as Renderer>::Node>,
    ) {
        self.state.mount(parent, marker);
    }

    fn insert_before_this(
        &self,
        parent: &<R as Renderer>::Element,
        child: &mut dyn Mountable<R>,
    ) -> bool {
        self.state.insert_before_this(parent, child)
    }
}
