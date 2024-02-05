use crate::{
    renderer::Renderer,
    view::{Position, Render, RenderHtml},
};
use std::marker::PhantomData;

pub mod attribute;
pub mod class;
pub mod element;
pub mod event;
pub mod islands;
pub mod node_ref;
pub mod property;
pub mod style;

pub struct Doctype<R: Renderer> {
    value: &'static str,
    rndr: PhantomData<R>,
}

pub fn doctype<R: Renderer>(value: &'static str) -> Doctype<R> {
    Doctype {
        value,
        rndr: PhantomData,
    }
}

impl<R: Renderer> Render<R> for Doctype<R> {
    type State = ();

    fn build(self) -> Self::State {}

    fn rebuild(self, _state: &mut Self::State) {}
}

impl<R> RenderHtml<R> for Doctype<R>
where
    R: Renderer,
    R::Element: Clone,
    R::Node: Clone,
{
    const MIN_LENGTH: usize = "<!DOCTYPE html>".len();

    fn to_html_with_buf(self, buf: &mut String, _position: &mut Position) {
        buf.push_str("<!DOCTYPE ");
        buf.push_str(self.value);
        buf.push('>');
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        _cursor: &crate::hydration::Cursor<R>,
        _position: &crate::view::PositionState,
    ) -> Self::State {
    }
}
