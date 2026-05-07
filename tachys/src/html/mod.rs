#[cfg(feature = "web")]
use self::attribute::Attribute;
#[cfg(feature = "web")]
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
#[cfg(feature = "web")]
use attribute::any_attribute::AnyAttribute;
#[cfg(feature = "web")]
use std::borrow::Cow;

/// Diagnostic message shared by event, directive, and property `.expect()` calls.
///
/// When the `ssr` feature is active, tachys skips creating client-side values
/// (event handlers, directives, properties) to avoid `SendWrapper` cross-thread
/// panics on multithreaded servers. If these `.expect()` calls fire, it means
/// the `ssr` feature was activated unintentionally via Cargo feature
/// unification in a client-side (CSR or hydrate) build.
///
/// Only referenced from the web-only event/directive/property modules.
pub(crate) const FEATURE_CONFLICT_DIAGNOSTIC: &str =
    "Value is None because the `ssr` feature is active. When `ssr` is \
     enabled, tachys skips creating client-side values (event handlers, \
     directives, properties) to avoid cross-thread panics on multithreaded \
     servers. If you are building a client-side (CSR or hydrate) target, this \
     means the `ssr` feature is being activated unintentionally via Cargo \
     feature unification; another dependency in your workspace is enabling \
     it. Run `cargo tree -e features -i tachys` to identify the source.";

/// Types for HTML attributes.
pub mod attribute;
/// Types for manipulating the `class` attribute and `classList`.
#[cfg(feature = "web")]
pub mod class;
/// Types for creating user-defined attributes with custom behavior (directives).
pub mod directive;
/// Types for HTML elements (web only). After the Phase 4 macro
/// refactor, native renderers expose their element constructors
/// through their own glue crate's `view_prelude::__leptos_view::elements`
/// namespace; `tachys::html::element` is unused on native.
#[cfg(feature = "web")]
pub mod element;

/// Types for DOM events. Web-only — native event descriptors live
/// in the per-renderer glue crate's `events` module (e.g.
/// `leptos_cocoa::events`).
#[cfg(feature = "web")]
pub mod event;
// Native event descriptors moved to the per-renderer glue crates
// (`leptos_cocoa::events`, `leptos_ios::events`, `leptos_gtk::events`)
// in Phase 5.
/// Types for adding interactive islands to inert HTML pages.
#[cfg(feature = "web")]
pub mod islands;
/// Types for accessing a reference to an HTML element.
#[cfg(feature = "web")]
pub mod node_ref;
/// Types for DOM properties.
#[cfg(feature = "web")]
pub mod property;
/// Types for the `style` attribute and individual style manipulation.
#[cfg(feature = "web")]
pub mod style;

/// A `<!DOCTYPE>` declaration. Web-only — disabled on native targets
/// since the renderer has no concept of inert HTML or doctypes.
#[cfg(feature = "web")]
pub struct Doctype {
    value: &'static str,
}

/// Creates a `<!DOCTYPE>`.
#[cfg(feature = "web")]
pub fn doctype(value: &'static str) -> Doctype {
    Doctype { value }
}

#[cfg(feature = "web")]
impl Render for Doctype {
    type State = ();

    fn build(self) -> Self::State {}

    fn rebuild(self, _state: &mut Self::State) {}
}

#[cfg(feature = "web")]
no_attrs!(Doctype);

#[cfg(feature = "web")]
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
#[cfg(feature = "web")]
pub struct InertElement {
    html: Cow<'static, str>,
}

#[cfg(feature = "web")]
impl InertElement {
    /// Creates a new inert element.
    pub fn new(html: impl Into<Cow<'static, str>>) -> Self {
        Self { html: html.into() }
    }
}

/// Retained view state for [`InertElement`].
#[cfg(feature = "web")]
pub struct InertElementState(Cow<'static, str>, Element);

#[cfg(feature = "web")]
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

#[cfg(feature = "web")]
impl Render for InertElement {
    type State = InertElementState;

    fn build(self) -> Self::State {
        let el = Rndr::create_element_from_html(self.html.clone());
        InertElementState(self.html, el)
    }

    fn rebuild(self, state: &mut Self::State) {
        let InertElementState(prev, el) = state;
        if &self.html != prev {
            let mut new_el = Rndr::create_element_from_html(self.html.clone());
            el.insert_before_this(&mut new_el);
            el.unmount();
            *el = new_el;
            *prev = self.html;
        }
    }
}

#[cfg(feature = "web")]
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

#[cfg(feature = "web")]
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
