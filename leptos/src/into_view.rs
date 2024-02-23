use leptos_dom::events::EventDescriptor;
use tachys::{
    html::attribute::global::OnAttribute,
    hydration::Cursor,
    renderer::{dom::Dom, Renderer},
    ssr::StreamBuilder,
    view::{Mountable, Position, PositionState, Render, RenderHtml},
};

pub struct View<T>(T)
where
    T: Sized;

pub trait IntoView: Sized + Render<Dom> + RenderHtml<Dom> {
    fn into_view(self) -> View<Self>;
}

impl<T: Render<Dom> + RenderHtml<Dom>> IntoView for T
where
    T: Sized,
{
    fn into_view(self) -> View<Self> {
        View(self)
    }
}

impl<T: Render<Dom>> Render<Dom> for View<T> {
    type State = T::State;
    type FallibleState = T::FallibleState;

    fn build(self) -> Self::State {
        self.0.build()
    }

    fn rebuild(self, state: &mut Self::State) {
        self.0.rebuild(state)
    }

    fn try_build(self) -> tachys::error::Result<Self::FallibleState> {
        self.0.try_build()
    }

    fn try_rebuild(
        self,
        state: &mut Self::FallibleState,
    ) -> tachys::error::Result<()> {
        self.0.try_rebuild(state)
    }
}

impl<T: RenderHtml<Dom>> RenderHtml<Dom> for View<T> {
    const MIN_LENGTH: usize = <T as RenderHtml<Dom>>::MIN_LENGTH;

    fn to_html_with_buf(self, buf: &mut String, position: &mut Position) {
        self.0.to_html_with_buf(buf, position);
    }

    fn to_html_async_with_buf<const OUT_OF_ORDER: bool>(
        self,
        buf: &mut StreamBuilder,
        position: &mut Position,
    ) where
        Self: Sized,
    {
        self.0.to_html_async_with_buf::<OUT_OF_ORDER>(buf, position)
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        cursor: &Cursor<Dom>,
        position: &PositionState,
    ) -> Self::State {
        self.0.hydrate::<FROM_SERVER>(cursor, position)
    }
}

/*pub trait IntoView {
    const MIN_HTML_LENGTH: usize;

    type State: Mountable<Dom>;

    fn build(self) -> Self::State;

    fn rebuild(self, state: &mut Self::State);

    fn to_html_with_buf(self, buf: &mut String, position: &mut Position);

    fn to_html_async_with_buf<const OUT_OF_ORDER: bool>(
        self,
        buf: &mut StreamBuilder,
        position: &mut Position,
    );

    fn hydrate<const FROM_SERVER: bool>(
        self,
        cursor: &Cursor<Dom>,
        position: &PositionState,
    ) -> Self::State;
}

impl<T: RenderHtml<Dom>> IntoView for T {}

impl<T: IntoView> Render<Dom> for T {
    type State = <Self as IntoView>::State;

    fn build(self) -> Self::State {
        IntoView::build(self)
    }

    fn rebuild(self, state: &mut Self::State) {
        IntoView::rebuild(self, state);
    }
}

impl<T: IntoView> RenderHtml<Dom> for T {
    const MIN_LENGTH: usize = T::MIN_HTML_LENGTH;

    fn to_html_with_buf(self, buf: &mut String, position: &mut Position) {
        IntoView::to_html_with_buf(self, buf, position);
    }

    fn to_html_async_with_buf<const OUT_OF_ORDER: bool>(
        self,
        buf: &mut StreamBuilder,
        position: &mut Position,
    ) where
        Self: Sized,
    {
        IntoView::to_html_async_with_buf::<OUT_OF_ORDER>(self, buf, position);
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        cursor: &Cursor<Dom>,
        position: &PositionState,
    ) -> Self::State {
        IntoView::hydrate::<FROM_SERVER>(self, cursor, position)
    }
}*/
