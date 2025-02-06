use super::{
    add_attr::AddAnyAttr, any_view::ExtraAttrsMut, Mountable, Position,
    PositionState, Render, RenderHtml, ToTemplate,
};
use crate::{
    html::attribute::{any_attribute::AnyAttribute, Attribute},
    hydration::Cursor,
    renderer::Rndr,
};

/// A view wrapper that uses a `<template>` node to optimize DOM node creation.
///
/// Rather than creating all of the DOM nodes each time it is built, this template will create a
/// single `<template>` node once, then use `.cloneNode(true)` to clone that entire tree, and
/// hydrate it to add event listeners and interactivity for this instance.
pub struct ViewTemplate<V> {
    view: V,
}

impl<V> ViewTemplate<V>
where
    V: Render + ToTemplate + 'static,
{
    /// Creates a new view template.
    pub fn new(view: V) -> Self {
        Self { view }
    }

    fn to_template() -> crate::renderer::types::TemplateElement {
        Rndr::get_template::<V>()
    }
}

impl<V> Render for ViewTemplate<V>
where
    V: Render + RenderHtml + ToTemplate + 'static,
    V::State: Mountable,
{
    type State = V::State;

    // TODO try_build/try_rebuild()

    fn build(self, extra_attrs: Option<Vec<AnyAttribute>>) -> Self::State {
        let tpl = Self::to_template();
        let contents = Rndr::clone_template(&tpl);
        self.view.hydrate::<false>(
            &Cursor::new(contents),
            &Default::default(),
            extra_attrs,
        )
    }

    fn rebuild(
        self,
        state: &mut Self::State,
        extra_attrs: Option<Vec<AnyAttribute>>,
    ) {
        self.view.rebuild(state, extra_attrs)
    }
}

impl<V> AddAnyAttr for ViewTemplate<V>
where
    V: RenderHtml + ToTemplate + 'static,
    V::State: Mountable,
{
    type Output<SomeNewAttr: Attribute> = ViewTemplate<V>;

    fn add_any_attr<NewAttr: Attribute>(
        self,
        _attr: NewAttr,
    ) -> Self::Output<NewAttr>
    where
        Self::Output<NewAttr>: RenderHtml,
    {
        panic!("AddAnyAttr not supported on ViewTemplate");
    }
}

impl<V> RenderHtml for ViewTemplate<V>
where
    V: RenderHtml + ToTemplate + 'static,
    V::State: Mountable,
{
    type AsyncOutput = V::AsyncOutput;
    type Owned = V::Owned;

    const MIN_LENGTH: usize = V::MIN_LENGTH;

    fn to_html_with_buf(
        self,
        buf: &mut String,
        position: &mut Position,
        escape: bool,
        mark_branches: bool,
        extra_attrs: Option<Vec<AnyAttribute>>,
    ) {
        self.view.to_html_with_buf(
            buf,
            position,
            escape,
            mark_branches,
            extra_attrs,
        )
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        cursor: &Cursor,
        position: &PositionState,
        extra_attrs: Option<Vec<AnyAttribute>>,
    ) -> Self::State {
        self.view
            .hydrate::<FROM_SERVER>(cursor, position, extra_attrs)
    }

    fn dry_resolve(&mut self, extra_attrs: ExtraAttrsMut<'_>) {
        self.view.dry_resolve(extra_attrs);
    }

    async fn resolve(
        self,
        extra_attrs: ExtraAttrsMut<'_>,
    ) -> Self::AsyncOutput {
        self.view.resolve(extra_attrs).await
    }

    fn into_owned(self) -> Self::Owned {
        self.view.into_owned()
    }
}

impl<V> ToTemplate for ViewTemplate<V>
where
    V: RenderHtml + ToTemplate + 'static,
    V::State: Mountable,
{
    const TEMPLATE: &'static str = V::TEMPLATE;

    fn to_template(
        buf: &mut String,
        class: &mut String,
        style: &mut String,
        inner_html: &mut String,
        position: &mut Position,
    ) {
        V::to_template(buf, class, style, inner_html, position);
    }
}
