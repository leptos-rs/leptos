use super::{
    add_attr::AddAnyAttr, Mountable, Position, PositionState, Render,
    RenderHtml, ToTemplate,
};
use crate::{
    html::attribute::Attribute, hydration::Cursor, renderer::DomRenderer,
};
use std::marker::PhantomData;

/// A view wrapper that uses a `<template>` node to optimize DOM node creation.
///
/// Rather than creating all of the DOM nodes each time it is built, this template will create a
/// single `<template>` node once, then use `.cloneNode(true)` to clone that entire tree, and
/// hydrate it to add event listeners and interactivity for this instance.
pub struct ViewTemplate<V, R> {
    view: V,
    rndr: PhantomData<R>,
}

impl<V, R> ViewTemplate<V, R>
where
    V: Render<R> + ToTemplate + 'static,
    R: DomRenderer,
{
    /// Creates a new view template.
    pub fn new(view: V) -> Self {
        Self {
            view,
            rndr: PhantomData,
        }
    }

    fn to_template() -> R::TemplateElement {
        R::get_template::<V>()
    }
}

impl<V, R> Render<R> for ViewTemplate<V, R>
where
    V: Render<R> + RenderHtml<R> + ToTemplate + 'static,
    V::State: Mountable<R>,
    R: DomRenderer,
{
    type State = V::State;

    // TODO try_build/try_rebuild()

    fn build(self) -> Self::State {
        let tpl = Self::to_template();
        let contents = R::clone_template(&tpl);
        self.view
            .hydrate::<false>(&Cursor::new(contents), &Default::default())
    }

    fn rebuild(self, state: &mut Self::State) {
        self.view.rebuild(state)
    }
}

impl<V, R> AddAnyAttr<R> for ViewTemplate<V, R>
where
    V: RenderHtml<R> + ToTemplate + 'static,
    V::State: Mountable<R>,
    R: DomRenderer,
{
    type Output<SomeNewAttr: Attribute<R>> = ViewTemplate<V, R>;

    fn add_any_attr<NewAttr: Attribute<R>>(
        self,
        _attr: NewAttr,
    ) -> Self::Output<NewAttr>
    where
        Self::Output<NewAttr>: RenderHtml<R>,
    {
        panic!("AddAnyAttr not supported on ViewTemplate");
    }
}

impl<V, R> RenderHtml<R> for ViewTemplate<V, R>
where
    V: RenderHtml<R> + ToTemplate + 'static,
    V::State: Mountable<R>,
    R: DomRenderer,
{
    type AsyncOutput = V::AsyncOutput;

    const MIN_LENGTH: usize = V::MIN_LENGTH;

    fn to_html_with_buf(
        self,
        buf: &mut String,
        position: &mut Position,
        escape: bool,
        mark_branches: bool,
    ) {
        self.view
            .to_html_with_buf(buf, position, escape, mark_branches)
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        cursor: &Cursor<R>,
        position: &PositionState,
    ) -> Self::State {
        self.view.hydrate::<FROM_SERVER>(cursor, position)
    }

    fn dry_resolve(&mut self) {
        todo!()
    }

    async fn resolve(self) -> Self::AsyncOutput {
        todo!()
    }
}

impl<V, R> ToTemplate for ViewTemplate<V, R>
where
    V: RenderHtml<R> + ToTemplate + 'static,
    V::State: Mountable<R>,
    R: DomRenderer,
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
