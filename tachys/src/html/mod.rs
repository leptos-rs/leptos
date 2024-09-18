use self::attribute::Attribute;
use crate::{
    hydration::Cursor,
    no_attrs,
    prelude::AddAnyAttr,
    renderer::{CastFrom, DomRenderer, Renderer},
    view::{Position, PositionState, Render, RenderHtml},
};
use std::{borrow::Cow, marker::PhantomData};

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
        _mark_branches: bool,
    ) {
        buf.push_str("<!DOCTYPE ");
        buf.push_str(self.value);
        buf.push('>');
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        _cursor: &Cursor<R>,
        _position: &PositionState,
    ) -> Self::State {
    }
}

/// An element that contains no interactivity, and whose contents can be known at compile time.
pub struct InertElement {
    html: Cow<'static, str>,
}

impl InertElement {
    /// Creates a new inert element.
    pub fn new(html: impl Into<Cow<'static, str>>) -> Self {
        Self { html: html.into() }
    }
}

impl<Rndr> Render<Rndr> for InertElement
where
    Rndr: DomRenderer,
{
    type State = Rndr::Element;

    fn build(self) -> Self::State {
        Rndr::create_element_from_html(&self.html)
    }

    fn rebuild(self, _state: &mut Self::State) {}
}

impl<Rndr> AddAnyAttr<Rndr> for InertElement
where
    Rndr: DomRenderer,
{
    type Output<SomeNewAttr: Attribute<Rndr>> = Self;

    fn add_any_attr<NewAttr: Attribute<Rndr>>(
        self,
        _attr: NewAttr,
    ) -> Self::Output<NewAttr>
    where
        Self::Output<NewAttr>: RenderHtml<Rndr>,
    {
        panic!(
            "InertElement does not support adding attributes. It should only \
             be used as a child, and not returned at the top level."
        )
    }
}

impl<Rndr> RenderHtml<Rndr> for InertElement
where
    Rndr: DomRenderer,
{
    type AsyncOutput = Self;

    const MIN_LENGTH: usize = 0;

    fn html_len(&self) -> usize {
        self.html.len()
    }

    fn dry_resolve(&mut self) {}

    async fn resolve(self) -> Self {
        self
    }

    fn to_html_with_buf(
        self,
        buf: &mut String,
        position: &mut Position,
        _escape: bool,
        _mark_branches: bool,
    ) {
        buf.push_str(&self.html);
        *position = Position::NextChild;
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        cursor: &Cursor<Rndr>,
        position: &PositionState,
    ) -> Self::State {
        let curr_position = position.get();
        if curr_position == Position::FirstChild {
            cursor.child();
        } else if curr_position != Position::Current {
            cursor.sibling();
        }
        let el = Rndr::Element::cast_from(cursor.current()).unwrap();
        position.set(Position::NextChild);
        el
    }
}
