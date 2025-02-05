use crate::ServerMetaContext;
use leptos::{
    attr::{
        any_attribute::{AnyAttribute, AnyAttributeState},
        NextAttribute,
    },
    component, html,
    reactive::owner::use_context,
    tachys::{
        dom::document,
        html::attribute::Attribute,
        hydration::Cursor,
        view::{
            add_attr::AddAnyAttr, any_view::ExtraAttrsMut, Mountable, Position,
            PositionState, Render, RenderHtml,
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
/// use leptos::prelude::*;
/// use leptos_meta::*;
///
/// #[component]
/// fn MyApp() -> impl IntoView {
///     provide_meta_context();
///     let (prefers_dark, set_prefers_dark) = signal(false);
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
    At: Attribute,
{
    attributes: At::State,
    extra_attrs: Option<Vec<AnyAttributeState>>,
}

impl<At> Render for BodyView<At>
where
    At: Attribute,
{
    type State = BodyViewState<At>;

    fn build(self, extra_attrs: Option<Vec<AnyAttribute>>) -> Self::State {
        let el = document().body().expect("there to be a <body> element");
        let attributes = self.attributes.build(&el);
        let extra_attrs = extra_attrs.map(|attrs| attrs.build(&el));
        BodyViewState {
            attributes,
            extra_attrs,
        }
    }

    fn rebuild(
        self,
        state: &mut Self::State,
        extra_attrs: Option<Vec<AnyAttribute>>,
    ) {
        self.attributes.rebuild(&mut state.attributes);
        if let (Some(extra_attrs), Some(extra_attr_states)) =
            (extra_attrs, &mut state.extra_attrs)
        {
            extra_attrs.rebuild(extra_attr_states);
        }
    }
}

impl<At> AddAnyAttr for BodyView<At>
where
    At: Attribute,
{
    type Output<SomeNewAttr: Attribute> =
        BodyView<<At as NextAttribute>::Output<SomeNewAttr>>;

    fn add_any_attr<NewAttr: Attribute>(
        self,
        attr: NewAttr,
    ) -> Self::Output<NewAttr>
    where
        Self::Output<NewAttr>: RenderHtml,
    {
        BodyView {
            attributes: self.attributes.add_any_attr(attr),
        }
    }
}

impl<At> RenderHtml for BodyView<At>
where
    At: Attribute,
{
    type AsyncOutput = BodyView<At::AsyncOutput>;
    type Owned = BodyView<At::CloneableOwned>;

    const MIN_LENGTH: usize = At::MIN_LENGTH;

    fn dry_resolve(&mut self, mut extra_attrs: ExtraAttrsMut<'_>) {
        self.attributes.dry_resolve();
        extra_attrs.iter_mut().for_each(Attribute::dry_resolve);
    }

    async fn resolve(
        self,
        extra_attrs: ExtraAttrsMut<'_>,
    ) -> Self::AsyncOutput {
        let (attributes, _) = futures::join!(
            self.attributes.resolve(),
            ExtraAttrsMut::resolve(extra_attrs)
        );
        BodyView { attributes }
    }

    fn to_html_with_buf(
        self,
        _buf: &mut String,
        _position: &mut Position,
        _escape: bool,
        _mark_branches: bool,
        extra_attrs: Option<Vec<AnyAttribute>>,
    ) {
        if let Some(meta) = use_context::<ServerMetaContext>() {
            let mut buf = String::new();
            _ = html::attributes_to_html(
                self.attributes,
                extra_attrs,
                &mut buf,
            );
            if !buf.is_empty() {
                _ = meta.body.send(buf);
            }
        }
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        _cursor: &Cursor,
        _position: &PositionState,
        extra_attrs: Option<Vec<AnyAttribute>>,
    ) -> Self::State {
        let el = document().body().expect("there to be a <body> element");
        let attributes = self.attributes.hydrate::<FROM_SERVER>(&el);
        let extra_attrs =
            extra_attrs.map(|attrs| attrs.hydrate::<FROM_SERVER>(&el));

        BodyViewState {
            attributes,
            extra_attrs,
        }
    }

    fn into_owned(self) -> Self::Owned {
        BodyView {
            attributes: self.attributes.into_cloneable_owned(),
        }
    }
}

impl<At> Mountable for BodyViewState<At>
where
    At: Attribute,
{
    fn unmount(&mut self) {}

    fn mount(
        &mut self,
        _parent: &leptos::tachys::renderer::types::Element,
        _marker: Option<&leptos::tachys::renderer::types::Node>,
    ) {
    }

    fn insert_before_this(&self, _child: &mut dyn Mountable) -> bool {
        false
    }
}
