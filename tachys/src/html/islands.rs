use super::attribute::Attribute;
use crate::{
    hydration::Cursor,
    prelude::{Render, RenderHtml},
    renderer::Renderer,
    ssr::StreamBuilder,
    view::{add_attr::AddAnyAttr, Position, PositionState},
};
use std::marker::PhantomData;

// TODO serialized props, too
/// An island of interactivity in an otherwise-inert HTML document.
pub struct Island<Rndr, View> {
    component: &'static str,
    view: View,
    rndr: PhantomData<Rndr>,
}
const ISLAND_TAG: &str = "leptos-island";
const ISLAND_CHILDREN_TAG: &str = "leptos-children";

impl<Rndr, View> Island<Rndr, View> {
    /// Creates a new island with the given component name.
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

    fn build(self) -> Self::State {
        self.view.build()
    }

    fn rebuild(self, state: &mut Self::State) {
        self.view.rebuild(state);
    }
}

impl<Rndr, View> AddAnyAttr<Rndr> for Island<Rndr, View>
where
    View: RenderHtml<Rndr>,
    Rndr: Renderer,
{
    type Output<SomeNewAttr: Attribute<Rndr>> =
        Island<Rndr, <View as AddAnyAttr<Rndr>>::Output<SomeNewAttr>>;

    fn add_any_attr<NewAttr: Attribute<Rndr>>(
        self,
        attr: NewAttr,
    ) -> Self::Output<NewAttr>
    where
        Self::Output<NewAttr>: RenderHtml<Rndr>,
    {
        let Island {
            component,
            view,
            rndr,
        } = self;
        Island {
            component,
            view: view.add_any_attr(attr),
            rndr,
        }
    }
}

impl<Rndr, View> RenderHtml<Rndr> for Island<Rndr, View>
where
    View: RenderHtml<Rndr>,
    Rndr: Renderer,
{
    type AsyncOutput = View::AsyncOutput;

    const MIN_LENGTH: usize = ISLAND_TAG.len() * 2
        + "<>".len()
        + "</>".len()
        + "data-component".len()
        + View::MIN_LENGTH;

    fn dry_resolve(&mut self) {
        self.view.dry_resolve()
    }

    async fn resolve(self) -> Self::AsyncOutput {
        self.view.resolve().await
    }

    fn to_html_with_buf(
        self,
        buf: &mut String,
        position: &mut Position,
        escape: bool,
        mark_branches: bool,
    ) {
        Self::open_tag(self.component, buf);
        self.view
            .to_html_with_buf(buf, position, escape, mark_branches);
        Self::close_tag(buf);
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
        // insert the opening tag synchronously
        let mut tag = String::new();
        Self::open_tag(self.component, &mut tag);
        buf.push_sync(&tag);

        // streaming render for the view
        self.view.to_html_async_with_buf::<OUT_OF_ORDER>(
            buf,
            position,
            escape,
            mark_branches,
        );

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

/// The children that will be projected into an [`Island`].
pub struct IslandChildren<Rndr, View> {
    view: View,
    rndr: PhantomData<Rndr>,
}

impl<Rndr, View> IslandChildren<Rndr, View> {
    /// Creates a new representation of the children.
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

    fn build(self) -> Self::State {}

    fn rebuild(self, _state: &mut Self::State) {}
}

impl<Rndr, View> AddAnyAttr<Rndr> for IslandChildren<Rndr, View>
where
    View: RenderHtml<Rndr>,
    Rndr: Renderer,
{
    type Output<SomeNewAttr: Attribute<Rndr>> =
        IslandChildren<Rndr, <View as AddAnyAttr<Rndr>>::Output<SomeNewAttr>>;

    fn add_any_attr<NewAttr: Attribute<Rndr>>(
        self,
        attr: NewAttr,
    ) -> Self::Output<NewAttr>
    where
        Self::Output<NewAttr>: RenderHtml<Rndr>,
    {
        let IslandChildren { view, rndr } = self;
        IslandChildren {
            view: view.add_any_attr(attr),
            rndr,
        }
    }
}

impl<Rndr, View> RenderHtml<Rndr> for IslandChildren<Rndr, View>
where
    View: RenderHtml<Rndr>,
    Rndr: Renderer,
{
    type AsyncOutput = View::AsyncOutput;

    const MIN_LENGTH: usize = ISLAND_CHILDREN_TAG.len() * 2
        + "<>".len()
        + "</>".len()
        + View::MIN_LENGTH;

    fn dry_resolve(&mut self) {
        self.view.dry_resolve()
    }

    async fn resolve(self) -> Self::AsyncOutput {
        // TODO should this be wrapped?
        self.view.resolve().await
    }

    fn to_html_with_buf(
        self,
        buf: &mut String,
        position: &mut Position,
        escape: bool,
        mark_branches: bool,
    ) {
        Self::open_tag(buf);
        self.view
            .to_html_with_buf(buf, position, escape, mark_branches);
        Self::close_tag(buf);
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
        // insert the opening tag synchronously
        let mut tag = String::new();
        Self::open_tag(&mut tag);
        buf.push_sync(&tag);

        // streaming render for the view
        self.view.to_html_async_with_buf::<OUT_OF_ORDER>(
            buf,
            position,
            escape,
            mark_branches,
        );

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
