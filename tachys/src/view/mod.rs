use self::add_attr::AddAnyAttr;
use crate::{hydration::Cursor, ssr::StreamBuilder};
use parking_lot::RwLock;
use std::{cell::RefCell, future::Future, rc::Rc, sync::Arc};

/// Add attributes to typed views.
pub mod add_attr;
/// A typed-erased view type.
pub mod any_view;
/// Allows choosing between one of several views.
pub mod either;
/// View rendering for `Result<_, _>` types.
pub mod error_boundary;
/// A type-erased view collection.
pub mod fragment;
/// View implementations for several iterable types.
pub mod iterators;
/// Keyed list iteration.
pub mod keyed;
mod primitives;
/// Optimized types for static strings known at compile time.
#[cfg(feature = "nightly")]
pub mod static_types;
/// View implementation for string types.
pub mod strings;
/// Optimizations for creating views via HTML `<template>` nodes.
pub mod template;
/// View implementations for tuples.
pub mod tuples;

/// The `Render` trait allows rendering something as part of the user interface.
///
/// It is generic over the renderer itself, as long as that implements the [`Renderer`]
/// trait.
pub trait Render: Sized {
    /// The “view state” for this type, which can be retained between updates.
    ///
    /// For example, for a text node, `State` might be the actual DOM text node
    /// and the previous string, to allow for diffing between updates.
    type State: Mountable;

    /// Creates the view for the first time, without hydrating from existing HTML.
    fn build(self) -> Self::State;

    /// Updates the view with new data.
    fn rebuild(self, state: &mut Self::State);
}

pub(crate) trait MarkBranch {
    fn open_branch(&mut self, branch_id: &str);

    fn close_branch(&mut self, branch_id: &str);
}

impl MarkBranch for String {
    fn open_branch(&mut self, branch_id: &str) {
        self.push_str("<!--bo-");
        self.push_str(branch_id);
        self.push_str("-->");
    }

    fn close_branch(&mut self, branch_id: &str) {
        self.push_str("<!--bc-");
        self.push_str(branch_id);
        self.push_str("-->");
    }
}

impl MarkBranch for StreamBuilder {
    fn open_branch(&mut self, branch_id: &str) {
        self.sync_buf.push_str("<!--bo-");
        self.sync_buf.push_str(branch_id);
        self.sync_buf.push_str("-->");
    }

    fn close_branch(&mut self, branch_id: &str) {
        self.sync_buf.push_str("<!--bc-");
        self.sync_buf.push_str(branch_id);
        self.sync_buf.push_str("-->");
    }
}

/// The `RenderHtml` trait allows rendering something to HTML, and transforming
/// that HTML into an interactive interface.
///
/// This process is traditionally called “server rendering” and “hydration.” As a
/// metaphor, this means that the structure of the view is created on the server, then
/// “dehydrated” to HTML, sent across the network, and “rehydrated” with interactivity
/// in the browser.
///
/// However, the same process can be done entirely in the browser: for example, a view
/// can be transformed into some HTML that is used to create a `<template>` node, which
/// can be cloned many times and “hydrated,” which is more efficient than creating the
/// whole view piece by piece.
pub trait RenderHtml
where
    Self: Render + AddAnyAttr + Send,
{
    /// The type of the view after waiting for all asynchronous data to load.
    type AsyncOutput: RenderHtml;

    /// The minimum length of HTML created when this view is rendered.
    const MIN_LENGTH: usize;

    /// Whether this should actually exist in the DOM, if it is the child of an element.
    const EXISTS: bool = true;

    /// “Runs” the view without other side effects. For primitive types, this is a no-op. For
    /// reactive types, this can be used to gather data about reactivity or about asynchronous data
    /// that needs to be loaded.
    fn dry_resolve(&mut self);

    /// Waits for any asynchronous sections of the view to load and returns the output.
    fn resolve(self) -> impl Future<Output = Self::AsyncOutput> + Send;

    /// An estimated length for this view, when rendered to HTML.
    ///
    /// This is used for calculating the string buffer size when rendering HTML. It does not need
    /// to be precise, but should be an appropriate estimate. The more accurate, the fewer
    /// reallocations will be required and the faster server-side rendering will be.
    fn html_len(&self) -> usize {
        Self::MIN_LENGTH
    }

    /// Renders a view to an HTML string.
    fn to_html(self) -> String
    where
        Self: Sized,
    {
        let mut buf = String::with_capacity(self.html_len());
        self.to_html_with_buf(&mut buf, &mut Position::FirstChild, true, false);
        buf
    }

    /// Renders a view to HTML with branch markers. This can be used to support libraries that diff
    /// HTML pages against one another, by marking sections of the view that branch to different
    /// types with marker comments.
    fn to_html_branching(self) -> String
    where
        Self: Sized,
    {
        let mut buf = String::with_capacity(self.html_len());
        self.to_html_with_buf(&mut buf, &mut Position::FirstChild, true, true);
        buf
    }

    /// Renders a view to an in-order stream of HTML.
    fn to_html_stream_in_order(self) -> StreamBuilder
    where
        Self: Sized,
    {
        let mut builder = StreamBuilder::with_capacity(self.html_len(), None);
        self.to_html_async_with_buf::<false>(
            &mut builder,
            &mut Position::FirstChild,
            true,
            false,
        );
        builder.finish()
    }

    /// Renders a view to an in-order stream of HTML with branch markers. This can be used to support libraries that diff
    /// HTML pages against one another, by marking sections of the view that branch to different
    /// types with marker comments.
    fn to_html_stream_in_order_branching(self) -> StreamBuilder
    where
        Self: Sized,
    {
        let mut builder = StreamBuilder::with_capacity(self.html_len(), None);
        self.to_html_async_with_buf::<false>(
            &mut builder,
            &mut Position::FirstChild,
            true,
            true,
        );
        builder.finish()
    }

    /// Renders a view to an out-of-order stream of HTML.
    fn to_html_stream_out_of_order(self) -> StreamBuilder
    where
        Self: Sized,
    {
        //let capacity = self.html_len();
        let mut builder =
            StreamBuilder::with_capacity(self.html_len(), Some(vec![0]));

        self.to_html_async_with_buf::<true>(
            &mut builder,
            &mut Position::FirstChild,
            true,
            false,
        );
        builder.finish()
    }

    /// Renders a view to an out-of-order stream of HTML with branch markers. This can be used to support libraries that diff
    /// HTML pages against one another, by marking sections of the view that branch to different
    /// types with marker comments.

    fn to_html_stream_out_of_order_branching(self) -> StreamBuilder
    where
        Self: Sized,
    {
        let mut builder =
            StreamBuilder::with_capacity(self.html_len(), Some(vec![0]));

        self.to_html_async_with_buf::<true>(
            &mut builder,
            &mut Position::FirstChild,
            true,
            true,
        );
        builder.finish()
    }

    /// Renders a view to HTML, writing it into the given buffer.
    fn to_html_with_buf(
        self,
        buf: &mut String,
        position: &mut Position,
        escape: bool,
        mark_branches: bool,
    );

    /// Renders a view into a buffer of (synchronous or asynchronous) HTML chunks.
    fn to_html_async_with_buf<const OUT_OF_ORDER: bool>(
        self,
        buf: &mut StreamBuilder,
        position: &mut Position,
        escape: bool,
        mark_branches: bool,
    ) where
        Self: Sized,
    {
        buf.with_buf(|buf| {
            self.to_html_with_buf(buf, position, escape, mark_branches)
        });
    }

    /// Makes a set of DOM nodes rendered from HTML interactive.
    ///
    /// If `FROM_SERVER` is `true`, this HTML was rendered using [`RenderHtml::to_html`]
    /// (e.g., during server-side rendering ).
    ///
    /// If `FROM_SERVER` is `false`, the HTML was rendered using [`ToTemplate::to_template`]
    /// (e.g., into a `<template>` element).
    fn hydrate<const FROM_SERVER: bool>(
        self,
        cursor: &Cursor,
        position: &PositionState,
    ) -> Self::State;

    /// Hydrates using [`RenderHtml::hydrate`], beginning at the given element.
    fn hydrate_from<const FROM_SERVER: bool>(
        self,
        el: &crate::renderer::types::Element,
    ) -> Self::State
    where
        Self: Sized,
    {
        self.hydrate_from_position::<FROM_SERVER>(el, Position::default())
    }

    /// Hydrates using [`RenderHtml::hydrate`], beginning at the given element and position.
    fn hydrate_from_position<const FROM_SERVER: bool>(
        self,
        el: &crate::renderer::types::Element,
        position: Position,
    ) -> Self::State
    where
        Self: Sized,
    {
        let cursor = Cursor::new(el.clone());
        let position = PositionState::new(position);
        self.hydrate::<FROM_SERVER>(&cursor, &position)
    }
}

/// Allows a type to be mounted to the DOM.
pub trait Mountable {
    /// Detaches the view from the DOM.
    fn unmount(&mut self);

    /// Mounts a node to the interface.
    fn mount(
        &mut self,
        parent: &crate::renderer::types::Element,
        marker: Option<&crate::renderer::types::Node>,
    );

    /// Inserts another `Mountable` type before this one. Returns `false` if
    /// this does not actually exist in the UI (for example, `()`).
    fn insert_before_this(&self, child: &mut dyn Mountable) -> bool;

    /// Inserts another `Mountable` type before this one, or before the marker
    /// if this one doesn't exist in the UI (for example, `()`).
    fn insert_before_this_or_marker(
        &self,
        parent: &crate::renderer::types::Element,
        child: &mut dyn Mountable,
        marker: Option<&crate::renderer::types::Node>,
    ) {
        if !self.insert_before_this(child) {
            child.mount(parent, marker);
        }
    }
}

/// Indicates where a node should be mounted to its parent.
pub enum MountKind {
    /// Node should be mounted before this marker node.
    Before(crate::renderer::types::Node),
    /// Node should be appended to the parent’s children.
    Append,
}

impl<T> Mountable for Option<T>
where
    T: Mountable,
{
    fn unmount(&mut self) {
        if let Some(ref mut mounted) = self {
            mounted.unmount()
        }
    }

    fn mount(
        &mut self,
        parent: &crate::renderer::types::Element,
        marker: Option<&crate::renderer::types::Node>,
    ) {
        if let Some(ref mut inner) = self {
            inner.mount(parent, marker);
        }
    }

    fn insert_before_this(&self, child: &mut dyn Mountable) -> bool {
        self.as_ref()
            .map(|inner| inner.insert_before_this(child))
            .unwrap_or(false)
    }
}

impl<T> Mountable for Rc<RefCell<T>>
where
    T: Mountable,
{
    fn unmount(&mut self) {
        self.borrow_mut().unmount()
    }

    fn mount(
        &mut self,
        parent: &crate::renderer::types::Element,
        marker: Option<&crate::renderer::types::Node>,
    ) {
        self.borrow_mut().mount(parent, marker);
    }

    fn insert_before_this(&self, child: &mut dyn Mountable) -> bool {
        self.borrow().insert_before_this(child)
    }
}

/// Allows data to be added to a static template.
pub trait ToTemplate {
    /// The HTML content of the static template.
    const TEMPLATE: &'static str = "";
    /// The `class` attribute content known at compile time.
    const CLASS: &'static str = "";
    /// The `style` attribute content known at compile time.
    const STYLE: &'static str = "";
    /// The length of the template.
    const LEN: usize = Self::TEMPLATE.as_bytes().len();

    /// Renders a view type to a template. This does not take actual view data,
    /// but can be used for constructing part of an HTML `<template>` that corresponds
    /// to a view of a particular type.
    fn to_template(
        buf: &mut String,
        class: &mut String,
        style: &mut String,
        inner_html: &mut String,
        position: &mut Position,
    );
}

/// Keeps track of what position the item currently being hydrated is in, relative to its siblings
/// and parents.
#[derive(Debug, Default, Clone)]
pub struct PositionState(Arc<RwLock<Position>>);

impl PositionState {
    /// Creates a new position tracker.
    pub fn new(position: Position) -> Self {
        Self(Arc::new(RwLock::new(position)))
    }

    /// Sets the current position.
    pub fn set(&self, position: Position) {
        *self.0.write() = position;
    }

    /// Gets the current position.
    pub fn get(&self) -> Position {
        *self.0.read()
    }

    /// Creates a new [`PositionState`], which starts with the same [`Position`], but no longer
    /// shares data with this `PositionState`.
    pub fn deep_clone(&self) -> Self {
        let current = self.get();
        Self(Arc::new(RwLock::new(current)))
    }
}

/// The position of this element, relative to others.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Default)]
pub enum Position {
    /// This is the current node.
    Current,
    /// This is the first child of its parent.
    #[default]
    FirstChild,
    /// This is the next child after another child.
    NextChild,
    /// This is the next child after a text node.
    NextChildAfterText,
    /// This is the only child of its parent.
    OnlyChild,
    /// This is the last child of its parent.
    LastChild,
}

/// Declares that this type can be converted into some other type, which can be renderered.
pub trait IntoRender {
    /// The renderable type into which this type can be converted.
    type Output;

    /// Consumes this value, transforming it into the renderable type.
    fn into_render(self) -> Self::Output;
}

impl<T> IntoRender for T
where
    T: Render,
{
    type Output = Self;

    fn into_render(self) -> Self::Output {
        self
    }
}
