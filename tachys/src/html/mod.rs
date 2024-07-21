use crate::{
    no_attrs,
    renderer::Renderer,
    view::{Position, Render, RenderHtml},
};
use std::marker::PhantomData;

/// Types for HTML attributes.
pub mod attribute;
/// Types for manipulating the `class` attribute and `classList`.
pub mod class;
/// Types for creating user-defined attributes with custom behavior (directives).
pub mod directive;
/// Types for HTML elements.
pub mod element;
/// Types for DOM events.
pub mod event;
/// Types for adding interactive islands to inert HTML pages.
pub mod islands;
/// Types for accessing a reference to an HTML element.
pub mod node_ref;
/// Types for DOM properties.
pub mod property;
/// Types for the `style` attribute and individual style manipulation.
pub mod style;

/// A `<!DOCTYPE>` declaration.
pub struct Doctype<R: Renderer> {
    value: &'static str,
    rndr: PhantomData<R>,
}

/// Creates a `<!DOCTYPE>`.
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

no_attrs!(Doctype<R>);

impl<R> RenderHtml<R> for Doctype<R>
where
    R: Renderer + Send,
{
    type AsyncOutput = Self;

    const MIN_LENGTH: usize = "<!DOCTYPE html>".len();

    fn dry_resolve(&mut self) {}

    async fn resolve(self) -> Self::AsyncOutput {
        self
    }

    fn to_html_with_buf(
        self,
        buf: &mut String,
        _position: &mut Position,
        _escape: bool,
        mark_branches: bool,
    ) {
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
