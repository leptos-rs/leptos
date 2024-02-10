//! Implements the [`Render`] and [`RenderHtml`] traits for signal guard types.

use crate::{prelude::RenderHtml, renderer::Renderer, view::Render};
use reactive_graph::signal::SignalReadGuard;

impl<T, Rndr> Render<Rndr> for SignalReadGuard<T>
where
    T: PartialEq + Clone + Render<Rndr>,
    Rndr: Renderer,
{
    type State = T::State;

    fn build(self) -> Self::State {
        todo!()
    }

    fn rebuild(self, state: &mut Self::State) {
        todo!()
    }
}

impl<T, Rndr> RenderHtml<Rndr> for SignalReadGuard<T>
where
    T: PartialEq + Clone + RenderHtml<Rndr>,
    Rndr: Renderer,
    Rndr::Element: Clone,
    Rndr::Node: Clone,
{
    const MIN_LENGTH: usize = T::MIN_LENGTH;

    fn to_html_with_buf(
        self,
        buf: &mut String,
        position: &mut crate::view::Position,
    ) {
        todo!()
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        cursor: &crate::hydration::Cursor<Rndr>,
        position: &crate::view::PositionState,
    ) -> Self::State {
        todo!()
    }
}
