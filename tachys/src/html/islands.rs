use crate::{
    hydration::Cursor,
    prelude::{Render, RenderHtml},
    renderer::Renderer,
    ssr::StreamBuilder,
    view::{Position, PositionState},
};
use std::marker::PhantomData;

// TODO serialized props, too
pub struct Island<Rndr, View> {
    component: &'static str,
    view: View,
    rndr: PhantomData<Rndr>,
}
const ISLAND_TAG: &str = "leptos-island";
const ISLAND_CHILDREN_TAG: &str = "leptos-children";

impl<Rndr, View> Island<Rndr, View> {
    pub fn new(component: &'static str, view: View) -> Self {
        Island {
            component,
            view,
            rndr: PhantomData,
        }
    }

    fn open_tag(component: &'static str, buf: &mut String) {
        buf.push('<');
        buf.push_str(ISLAND_TAG);
        buf.push(' ');
        buf.push_str("data-component=\"");
        buf.push_str(component);
        buf.push_str("\">");
        // TODO insert serialized props
    }

    fn close_tag(buf: &mut String) {
        buf.push_str("</");
        buf.push_str(ISLAND_TAG);
        buf.push('>');
    }
}

impl<Rndr, View> Render<Rndr> for Island<Rndr, View>
where
    View: Render<Rndr>,
    Rndr: Renderer,
{
    type State = View::State;
    type FallibleState = View::FallibleState;
    type AsyncOutput = View::AsyncOutput;

    fn build(self) -> Self::State {
        self.view.build()
    }

    fn rebuild(self, state: &mut Self::State) {
        self.view.rebuild(state);
    }

    fn try_build(self) -> any_error::Result<Self::FallibleState> {
        self.view.try_build()
    }

    fn try_rebuild(
        self,
        state: &mut Self::FallibleState,
    ) -> any_error::Result<()> {
        self.view.try_rebuild(state)
    }

    async fn resolve(self) -> Self::AsyncOutput {
        self.view.resolve().await
    }
}

impl<Rndr, View> RenderHtml<Rndr> for Island<Rndr, View>
where
    View: RenderHtml<Rndr>,
    Rndr: Renderer,
{
    const MIN_LENGTH: usize = ISLAND_TAG.len() * 2
        + "<>".len()
        + "</>".len()
        + "data-component".len()
        + View::MIN_LENGTH;

    fn to_html_with_buf(self, buf: &mut String, position: &mut Position) {
        Self::open_tag(self.component, buf);
        self.view.to_html_with_buf(buf, position);
        Self::close_tag(buf);
    }

    fn to_html_async_with_buf<const OUT_OF_ORDER: bool>(
        self,
        buf: &mut StreamBuilder,
        position: &mut Position,
    ) where
        Self: Sized,
    {
        // insert the opening tag synchronously
        let mut tag = String::new();
        Self::open_tag(self.component, &mut tag);
        buf.push_sync(&tag);

        // streaming render for the view
        self.view
            .to_html_async_with_buf::<OUT_OF_ORDER>(buf, position);

        // and insert the closing tag synchronously
        tag.clear();
        Self::close_tag(&mut tag);
        buf.push_sync(&tag);
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        cursor: &Cursor<Rndr>,
        position: &PositionState,
    ) -> Self::State {
        position.set(Position::FirstChild);
        self.view.hydrate::<FROM_SERVER>(cursor, position)
    }
}

pub struct IslandChildren<Rndr, View> {
    view: View,
    rndr: PhantomData<Rndr>,
}

impl<Rndr, View> IslandChildren<Rndr, View> {
    pub fn new(view: View) -> Self {
        IslandChildren {
            view,
            rndr: PhantomData,
        }
    }

    fn open_tag(buf: &mut String) {
        buf.push('<');
        buf.push_str(ISLAND_CHILDREN_TAG);
        buf.push('>');
    }

    fn close_tag(buf: &mut String) {
        buf.push_str("</");
        buf.push_str(ISLAND_CHILDREN_TAG);
        buf.push('>');
    }
}

impl<Rndr, View> Render<Rndr> for IslandChildren<Rndr, View>
where
    View: Render<Rndr>,
    Rndr: Renderer,
{
    type State = ();
    type FallibleState = Self::State;
    type AsyncOutput = View::AsyncOutput;

    fn build(self) -> Self::State {}

    fn rebuild(self, _state: &mut Self::State) {}

    fn try_build(self) -> any_error::Result<Self::FallibleState> {
        Ok(())
    }

    fn try_rebuild(
        self,
        _state: &mut Self::FallibleState,
    ) -> any_error::Result<()> {
        Ok(())
    }

    async fn resolve(self) -> Self::AsyncOutput {
        // TODO should this be wrapped?
        self.view.resolve().await
    }
}

impl<Rndr, View> RenderHtml<Rndr> for IslandChildren<Rndr, View>
where
    View: RenderHtml<Rndr>,
    Rndr: Renderer,
{
    const MIN_LENGTH: usize = ISLAND_CHILDREN_TAG.len() * 2
        + "<>".len()
        + "</>".len()
        + View::MIN_LENGTH;

    fn to_html_with_buf(self, buf: &mut String, position: &mut Position) {
        Self::open_tag(buf);
        self.view.to_html_with_buf(buf, position);
        Self::close_tag(buf);
    }

    fn to_html_async_with_buf<const OUT_OF_ORDER: bool>(
        self,
        buf: &mut StreamBuilder,
        position: &mut Position,
    ) where
        Self: Sized,
    {
        // insert the opening tag synchronously
        let mut tag = String::new();
        Self::open_tag(&mut tag);
        buf.push_sync(&tag);

        // streaming render for the view
        self.view
            .to_html_async_with_buf::<OUT_OF_ORDER>(buf, position);

        // and insert the closing tag synchronously
        tag.clear();
        Self::close_tag(&mut tag);
        buf.push_sync(&tag);
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        cursor: &Cursor<Rndr>,
        position: &PositionState,
    ) -> Self::State {
        // island children aren't hydrated
        // we update the walk to pass over them
        // but we don't hydrate their children
        let curr_position = position.get();
        if curr_position == Position::FirstChild {
            cursor.child();
        } else if curr_position != Position::Current {
            cursor.sibling();
        }
    }
}
