use crate::ServerMetaContext;
use leptos::{
    attr::NextAttribute,
    component, html,
    reactive_graph::owner::use_context,
    tachys::{
        dom::document,
        html::attribute::Attribute,
        hydration::Cursor,
        renderer::{dom::Dom, Renderer},
        view::{
            add_attr::AddAnyAttr, Mountable, Position, PositionState, Render,
            RenderHtml,
        },
    },
    IntoView,
};

/// A component to set metadata on the document’s `<body>` element from
/// within the application.
///
/// This component takes no props, but can take any number of spread attributes
/// following the `{..}` operator.
///
/// ```
/// use leptos::*;
/// use leptos_meta::*;
///
/// #[component]
/// fn MyApp() -> impl IntoView {
///     provide_meta_context();
///     let (prefers_dark, set_prefers_dark) = create_signal(false);
///     let body_class = move || {
///         if prefers_dark.get() {
///             "dark".to_string()
///         } else {
///             "light".to_string()
///         }
///     };
///
///     view! {
///       <main>
///         <Body {..} class=body_class id="body"/>
///       </main>
///     }
/// }
/// ```
#[component]
pub fn Body() -> impl IntoView {
    BodyView { attributes: () }
}

struct BodyView<At> {
    attributes: At,
}

struct BodyViewState<At>
where
    At: Attribute<Dom>,
{
    attributes: At::State,
}

impl<At> Render<Dom> for BodyView<At>
where
    At: Attribute<Dom>,
{
    type State = BodyViewState<At>;

    fn build(self) -> Self::State {
        let el = document().body().expect("there to be a <body> element");
        let attributes = self.attributes.build(&el);

        BodyViewState { attributes }
    }

    fn rebuild(self, state: &mut Self::State) {
        self.attributes.rebuild(&mut state.attributes);
    }
}

impl<At> AddAnyAttr<Dom> for BodyView<At>
where
    At: Attribute<Dom>,
{
    type Output<SomeNewAttr: Attribute<Dom>> =
        BodyView<<At as NextAttribute<Dom>>::Output<SomeNewAttr>>;

    fn add_any_attr<NewAttr: Attribute<Dom>>(
        self,
        attr: NewAttr,
    ) -> Self::Output<NewAttr>
    where
        Self::Output<NewAttr>: RenderHtml<Dom>,
    {
        BodyView {
            attributes: self.attributes.add_any_attr(attr),
        }
    }
}

impl<At> RenderHtml<Dom> for BodyView<At>
where
    At: Attribute<Dom>,
{
    type AsyncOutput = BodyView<At::AsyncOutput>;

    const MIN_LENGTH: usize = At::MIN_LENGTH;

    fn dry_resolve(&mut self) {
        self.attributes.dry_resolve();
    }

    async fn resolve(self) -> Self::AsyncOutput {
        BodyView {
            attributes: self.attributes.resolve().await,
        }
    }

    fn to_html_with_buf(
        self,
        _buf: &mut String,
        _position: &mut Position,
        _escape: bool,
        _mark_branches: bool,
    ) {
        if let Some(meta) = use_context::<ServerMetaContext>() {
            let mut buf = String::new();
            _ = html::attributes_to_html(self.attributes, &mut buf);
            if !buf.is_empty() {
                _ = meta.body.send(buf);
            }
        }
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        _cursor: &Cursor<Dom>,
        _position: &PositionState,
    ) -> Self::State {
        let el = document().body().expect("there to be a <body> element");
        let attributes = self.attributes.hydrate::<FROM_SERVER>(&el);

        BodyViewState { attributes }
    }
}

impl<At> Mountable<Dom> for BodyViewState<At>
where
    At: Attribute<Dom>,
{
    fn unmount(&mut self) {}

    fn mount(
        &mut self,
        _parent: &<Dom as Renderer>::Element,
        _marker: Option<&<Dom as Renderer>::Node>,
    ) {
    }

    fn insert_before_this(&self, _child: &mut dyn Mountable<Dom>) -> bool {
        false
    }
}
