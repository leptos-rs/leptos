use self::attribute::Attribute;
use crate::{
    hydration::Cursor,
    no_attrs,
    prelude::{AddAnyAttr, Mountable},
    renderer::{
        dom::{Element, Node},
        CastFrom, Rndr,
    },
    view::{Position, PositionState, Render, RenderHtml},
};
use attribute::any_attribute::AnyAttribute;
use std::borrow::Cow;

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
pub struct Doctype {
    value: &'static str,
}

/// Creates a `<!DOCTYPE>`.
pub fn doctype(value: &'static str) -> Doctype {
    Doctype { value }
}

impl Render for Doctype {
    type State = ();

    fn build(self) -> Self::State {}

    fn rebuild(self, _state: &mut Self::State) {}
}

no_attrs!(Doctype);

impl RenderHtml for Doctype {
    type AsyncOutput = Self;
    type Owned = Self;

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
        _extra_attrs: Vec<AnyAttribute>,
    ) {
        buf.push_str("<!DOCTYPE ");
        buf.push_str(self.value);
        buf.push('>');
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        _cursor: &Cursor,
        _position: &PositionState,
    ) -> Self::State {
    }

    fn into_owned(self) -> Self::Owned {
        self
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

/// Retained view state for [`InertElement`].
pub struct InertElementState(Cow<'static, str>, Element);

impl Mountable for InertElementState {
    fn unmount(&mut self) {
        self.1.unmount();
    }

    fn mount(&mut self, parent: &Element, marker: Option<&Node>) {
        self.1.mount(parent, marker)
    }

    fn insert_before_this(&self, child: &mut dyn Mountable) -> bool {
        self.1.insert_before_this(child)
    }

    fn elements(&self) -> Vec<crate::renderer::types::Element> {
        vec![self.1.clone()]
    }
}

impl Render for InertElement {
    type State = InertElementState;

    fn build(self) -> Self::State {
        let el = Rndr::create_element_from_html(&self.html);
        InertElementState(self.html, el)
    }

    fn rebuild(self, state: &mut Self::State) {
        let InertElementState(prev, el) = state;
        if &self.html != prev {
            let mut new_el = Rndr::create_element_from_html(&self.html);
            el.insert_before_this(&mut new_el);
            el.unmount();
            *el = new_el;
            *prev = self.html;
        }
    }
}

impl AddAnyAttr for InertElement {
    type Output<SomeNewAttr: Attribute> = Self;

    fn add_any_attr<NewAttr: Attribute>(
        self,
        _attr: NewAttr,
    ) -> Self::Output<NewAttr>
    where
        Self::Output<NewAttr>: RenderHtml,
    {
        panic!(
            "InertElement does not support adding attributes. It should only \
             be used as a child, and not returned at the top level."
        )
    }
}

impl RenderHtml for InertElement {
    type AsyncOutput = Self;
    type Owned = Self;

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
        _extra_attrs: Vec<AnyAttribute>,
    ) {
        buf.push_str(&self.html);
        *position = Position::NextChild;
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        cursor: &Cursor,
        position: &PositionState,
    ) -> Self::State {
        let curr_position = position.get();
        if curr_position == Position::FirstChild {
            cursor.child();
        } else if curr_position != Position::Current {
            cursor.sibling();
        }
        let el = crate::renderer::types::Element::cast_from(cursor.current())
            .unwrap();
        position.set(Position::NextChild);
        InertElementState(self.html, el)
    }

    fn into_owned(self) -> Self::Owned {
        self
    }
}
