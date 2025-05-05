use super::attribute::{any_attribute::AnyAttribute, Attribute};
use crate::{
    hydration::Cursor,
    prelude::{Render, RenderHtml},
    ssr::StreamBuilder,
    view::{add_attr::AddAnyAttr, Position, PositionState},
};

/// An island of interactivity in an otherwise-inert HTML document.
pub struct Island<View> {
    has_element_representation: bool,
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
            has_element_representation:
                Self::should_have_element_representation(),
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

    /// Whether this island should be represented by an actual HTML element
    fn should_have_element_representation() -> bool {
        #[cfg(feature = "reactive_graph")]
        {
            use reactive_graph::owner::{use_context, IsHydrating};
            let already_hydrating =
                use_context::<IsHydrating>().map(|h| h.0).unwrap_or(false);
            !already_hydrating
        }
        #[cfg(not(feature = "reactive_graph"))]
        {
            true
        }
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
            has_element_representation,
            component,
            props_json,
            view,
        } = self;
        Island {
            has_element_representation,
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
    type Owned = Island<View::Owned>;

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
            has_element_representation,
            component,
            props_json,
            view,
        } = self;
        Island {
            has_element_representation,
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
        extra_attrs: Vec<AnyAttribute>,
    ) {
        let has_element = self.has_element_representation;
        if has_element {
            Self::open_tag(self.component, &self.props_json, buf);
        }
        self.view.to_html_with_buf(
            buf,
            position,
            escape,
            mark_branches,
            extra_attrs,
        );
        if has_element {
            Self::close_tag(buf);
        }
    }

    fn to_html_async_with_buf<const OUT_OF_ORDER: bool>(
        self,
        buf: &mut StreamBuilder,
        position: &mut Position,
        escape: bool,
        mark_branches: bool,
        extra_attrs: Vec<AnyAttribute>,
    ) where
        Self: Sized,
    {
        let has_element = self.has_element_representation;
        // insert the opening tag synchronously
        let mut tag = String::new();
        if has_element {
            Self::open_tag(self.component, &self.props_json, &mut tag);
        }
        buf.push_sync(&tag);

        // streaming render for the view
        self.view.to_html_async_with_buf::<OUT_OF_ORDER>(
            buf,
            position,
            escape,
            mark_branches,
            extra_attrs,
        );

        // and insert the closing tag synchronously
        tag.clear();
        if has_element {
            Self::close_tag(&mut tag);
        }
        buf.push_sync(&tag);
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        cursor: &Cursor,
        position: &PositionState,
    ) -> Self::State {
        if self.has_element_representation {
            if position.get() == Position::FirstChild {
                cursor.child();
            } else if position.get() == Position::NextChild {
                cursor.sibling();
            }
            position.set(Position::FirstChild);
        }

        self.view.hydrate::<FROM_SERVER>(cursor, position)
    }

    fn into_owned(self) -> Self::Owned {
        Island {
            has_element_representation: self.has_element_representation,
            component: self.component,
            props_json: self.props_json,
            view: self.view.into_owned(),
        }
    }
}

/// The children that will be projected into an [`Island`].
pub struct IslandChildren<View> {
    view: View,
    on_hydrate: Option<Box<dyn Fn() + Send + Sync>>,
}

impl<View> IslandChildren<View> {
    /// Creates a new representation of the children.
    pub fn new(view: View) -> Self {
        IslandChildren {
            view,
            on_hydrate: None,
        }
    }

    /// Creates a new representation of the children, with a function to be called whenever
    /// a child island hydrates.
    pub fn new_with_on_hydrate(
        view: View,
        on_hydrate: impl Fn() + Send + Sync + 'static,
    ) -> Self {
        IslandChildren {
            view,
            on_hydrate: Some(Box::new(on_hydrate)),
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
        let IslandChildren { view, on_hydrate } = self;
        IslandChildren {
            view: view.add_any_attr(attr),
            on_hydrate,
        }
    }
}

impl<View> RenderHtml for IslandChildren<View>
where
    View: RenderHtml,
{
    type AsyncOutput = IslandChildren<View::AsyncOutput>;
    type Owned = IslandChildren<View::Owned>;

    const MIN_LENGTH: usize = ISLAND_CHILDREN_TAG.len() * 2
        + "<>".len()
        + "</>".len()
        + View::MIN_LENGTH;

    fn dry_resolve(&mut self) {
        self.view.dry_resolve()
    }

    async fn resolve(self) -> Self::AsyncOutput {
        let IslandChildren { view, on_hydrate } = self;
        IslandChildren {
            view: view.resolve().await,
            on_hydrate,
        }
    }

    fn to_html_with_buf(
        self,
        buf: &mut String,
        position: &mut Position,
        escape: bool,
        mark_branches: bool,
        extra_attrs: Vec<AnyAttribute>,
    ) {
        Self::open_tag(buf);
        self.view.to_html_with_buf(
            buf,
            position,
            escape,
            mark_branches,
            extra_attrs,
        );
        Self::close_tag(buf);
    }

    fn to_html_async_with_buf<const OUT_OF_ORDER: bool>(
        self,
        buf: &mut StreamBuilder,
        position: &mut Position,
        escape: bool,
        mark_branches: bool,
        extra_attrs: Vec<AnyAttribute>,
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
            extra_attrs,
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
        position.set(Position::NextChild);

        if let Some(on_hydrate) = self.on_hydrate {
            use crate::{
                hydration::failed_to_cast_element, renderer::CastFrom,
            };

            let el =
                crate::renderer::types::Element::cast_from(cursor.current())
                    .unwrap_or_else(|| {
                        failed_to_cast_element(
                            "leptos-children",
                            cursor.current(),
                        )
                    });
            let cb = wasm_bindgen::closure::Closure::wrap(
                on_hydrate as Box<dyn Fn()>,
            );
            _ = js_sys::Reflect::set(
                &el,
                &wasm_bindgen::JsValue::from_str("$$on_hydrate"),
                &cb.into_js_value(),
            );
        }
    }

    fn into_owned(self) -> Self::Owned {
        IslandChildren {
            view: self.view.into_owned(),
            on_hydrate: self.on_hydrate,
        }
    }
}
