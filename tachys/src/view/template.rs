use super::{
    Mountable, Position, PositionState, Render, RenderHtml, ToTemplate,
};
use crate::{hydration::Cursor, renderer::DomRenderer};
use std::marker::PhantomData;

pub struct ViewTemplate<V, R>
where
    V: Render<R> + ToTemplate,
    R: DomRenderer,
{
    view: V,
    rndr: PhantomData<R>,
}

impl<V, R> ViewTemplate<V, R>
where
    V: Render<R> + ToTemplate + 'static,
    R: DomRenderer,
{
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
    type FallibleState = V::FallibleState;
    type AsyncOutput = Self;

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

    fn try_build(self) -> any_error::Result<Self::FallibleState> {
        todo!()
    }

    fn try_rebuild(
        self,
        _state: &mut Self::FallibleState,
    ) -> any_error::Result<()> {
        todo!()
    }

    async fn resolve(self) -> Self::AsyncOutput {
        self
    }
}

impl<V, R> RenderHtml<R> for ViewTemplate<V, R>
where
    V: RenderHtml<R> + ToTemplate + 'static,
    V::State: Mountable<R>,
    R: DomRenderer,
{
    const MIN_LENGTH: usize = V::MIN_LENGTH;

    fn to_html_with_buf(self, buf: &mut String, position: &mut Position) {
        self.view.to_html_with_buf(buf, position)
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        cursor: &Cursor<R>,
        position: &PositionState,
    ) -> Self::State {
        self.view.hydrate::<FROM_SERVER>(cursor, position)
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
