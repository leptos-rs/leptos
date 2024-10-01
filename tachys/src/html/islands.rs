use super::attribute::Attribute;
use crate::{
    hydration::Cursor,
    prelude::{Render, RenderHtml},
    ssr::StreamBuilder,
    view::{add_attr::AddAnyAttr, Position, PositionState},
};

/// An island of interactivity in an otherwise-inert HTML document.
pub struct Island<View> {
    component: &'static str,
    props_json: String,
    view: View,
}
const ISLAND_TAG: &str = "leptos-island";
const ISLAND_CHILDREN_TAG: &str = "leptos-children";

impl<View> Island<View> {
    /// Creates a new island with the given component name.
    pub fn new(component: &'static str, view: View) -> Self {
        Island {
            component,
            props_json: String::new(),
            view,
        }
    }

    /// Adds serialized component props as JSON.
    pub fn with_props(mut self, props_json: String) -> Self {
        self.props_json = props_json;
        self
    }

    fn open_tag(component: &'static str, props: &str, buf: &mut String) {
        buf.push('<');
        buf.push_str(ISLAND_TAG);
        buf.push(' ');
        buf.push_str("data-component=\"");
        buf.push_str(component);
        buf.push('"');
        if !props.is_empty() {
            buf.push_str(" data-props=\"");
            buf.push_str(&html_escape::encode_double_quoted_attribute(&props));
            buf.push('"');
        }
        buf.push('>');
    }

    fn close_tag(buf: &mut String) {
        buf.push_str("</");
        buf.push_str(ISLAND_TAG);
        buf.push('>');
    }
}

impl<View> Render for Island<View>
where
    View: Render,
{
    type State = View::State;

    fn build(self) -> Self::State {
        self.view.build()
    }

    fn rebuild(self, state: &mut Self::State) {
        self.view.rebuild(state);
    }
}

impl<View> AddAnyAttr for Island<View>
where
    View: RenderHtml,
{
    type Output<SomeNewAttr: Attribute> =
        Island<<View as AddAnyAttr>::Output<SomeNewAttr>>;

    fn add_any_attr<NewAttr: Attribute>(
        self,
        attr: NewAttr,
    ) -> Self::Output<NewAttr>
    where
        Self::Output<NewAttr>: RenderHtml,
    {
        let Island {
            component,
            props_json,
            view,
        } = self;
        Island {
            component,
            props_json,
            view: view.add_any_attr(attr),
        }
    }
}

impl<View> RenderHtml for Island<View>
where
    View: RenderHtml,
{
    type AsyncOutput = Island<View::AsyncOutput>;

    const MIN_LENGTH: usize = ISLAND_TAG.len() * 2
        + "<>".len()
        + "</>".len()
        + "data-component".len()
        + View::MIN_LENGTH;

    fn dry_resolve(&mut self) {
        self.view.dry_resolve()
    }

    async fn resolve(self) -> Self::AsyncOutput {
        let Island {
            component,
            props_json,
            view,
        } = self;
        Island {
            component,
            props_json,
            view: view.resolve().await,
        }
    }

    fn to_html_with_buf(
        self,
        buf: &mut String,
        position: &mut Position,
        escape: bool,
        mark_branches: bool,
    ) {
        Self::open_tag(self.component, &self.props_json, buf);
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
        Self::open_tag(self.component, &self.props_json, &mut tag);
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
        cursor: &Cursor,
        position: &PositionState,
    ) -> Self::State {
        if position.get() == Position::FirstChild {
            cursor.child();
        } else if position.get() == Position::NextChild {
            cursor.sibling();
        }
        position.set(Position::FirstChild);
        self.view.hydrate::<FROM_SERVER>(cursor, position)
    }
}

/// The children that will be projected into an [`Island`].
pub struct IslandChildren<View> {
    view: View,
}

impl<View> IslandChildren<View> {
    /// Creates a new representation of the children.
    pub fn new(view: View) -> Self {
        IslandChildren { view }
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

impl<View> Render for IslandChildren<View>
where
    View: Render,
{
    type State = ();

    fn build(self) -> Self::State {}

    fn rebuild(self, _state: &mut Self::State) {}
}

impl<View> AddAnyAttr for IslandChildren<View>
where
    View: RenderHtml,
{
    type Output<SomeNewAttr: Attribute> =
        IslandChildren<<View as AddAnyAttr>::Output<SomeNewAttr>>;

    fn add_any_attr<NewAttr: Attribute>(
        self,
        attr: NewAttr,
    ) -> Self::Output<NewAttr>
    where
        Self::Output<NewAttr>: RenderHtml,
    {
        let IslandChildren { view } = self;
        IslandChildren {
            view: view.add_any_attr(attr),
        }
    }
}

impl<View> RenderHtml for IslandChildren<View>
where
    View: RenderHtml,
{
    type AsyncOutput = IslandChildren<View::AsyncOutput>;

    const MIN_LENGTH: usize = ISLAND_CHILDREN_TAG.len() * 2
        + "<>".len()
        + "</>".len()
        + View::MIN_LENGTH;

    fn dry_resolve(&mut self) {
        self.view.dry_resolve()
    }

    async fn resolve(self) -> Self::AsyncOutput {
        let IslandChildren { view } = self;
        IslandChildren {
            view: view.resolve().await,
        }
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
        cursor: &Cursor,
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
