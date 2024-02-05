use crate::{hydration::Cursor, renderer::Renderer, ssr::StreamBuilder};
use parking_lot::RwLock;
use std::sync::Arc;

pub mod any_view;
pub mod either;
pub mod error_boundary;
pub mod iterators;
pub mod keyed;
mod primitives;
#[cfg(feature = "nightly")]
pub mod static_types;
pub mod strings;
pub mod template;
pub mod tuples;

/// The `Render` trait allows rendering something as part of the user interface.
///
/// It is generic over the renderer itself, as long as that implements the [`Renderer`]
/// trait.
pub trait Render<R: Renderer> {
    /// The “view state” for this type, which can be retained between updates.
    ///
    /// For example, for a text node, `State` might be the actual DOM text node
    /// and the previous string, to allow for diffing between updates.
    type State: Mountable<R>;

    /// Creates the view for the first time, without hydrating from existing HTML.
    fn build(self) -> Self::State;

    /// Updates the view with new data.
    fn rebuild(self, state: &mut Self::State);
}

pub trait InfallibleRender {}

pub trait FallibleRender<R>: Sized + Render<R>
where
    R: Renderer,
{
    type FallibleState: Mountable<R>;
    type Error;

    /// Creates the view fallibly, handling any [`Result`] by propagating its `Err`.
    fn try_build(self) -> Result<Self::FallibleState, Self::Error>;

    /// Updates the view with new data fallibly, handling any [`Result`] by propagating its `Err`.
    fn try_rebuild(
        self,
        state: &mut Self::FallibleState,
    ) -> Result<(), Self::Error>;
}

#[derive(Debug, Clone, Copy)]
pub struct NeverError;

impl core::fmt::Display for NeverError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        Ok(())
    }
}

impl std::error::Error for NeverError {}

impl<T, R> FallibleRender<R> for T
where
    T: Render<R> + InfallibleRender,
    R: Renderer,
{
    type FallibleState = Self::State;
    type Error = NeverError;

    fn try_build(self) -> Result<Self::FallibleState, Self::Error> {
        Ok(self.build())
    }

    fn try_rebuild(
        self,
        state: &mut Self::FallibleState,
    ) -> Result<(), Self::Error> {
        self.rebuild(state);
        Ok(())
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
pub trait RenderHtml<R: Renderer>
where
    Self: Render<R>,
    R::Node: Clone,
    R::Element: Clone,
{
    const MIN_LENGTH: usize;

    fn min_length(&self) -> usize {
        Self::MIN_LENGTH
    }

    /// Renders a view to an HTML string.
    fn to_html(self) -> String
    where
        Self: Sized,
    {
        let mut buf = String::with_capacity(Self::MIN_LENGTH);
        self.to_html_with_buf(&mut buf, &mut Position::FirstChild);
        buf
    }

    /// Renders a view to an in-order stream of HTML.
    fn to_html_stream_in_order(self) -> StreamBuilder
    where
        Self: Sized,
    {
        let mut builder = StreamBuilder::default();
        self.to_html_async_with_buf::<false>(
            &mut builder,
            &mut Position::FirstChild,
        );
        builder.finish()
    }

    /// Renders a view to an out-of-order stream of HTML.
    fn to_html_stream_out_of_order(self) -> StreamBuilder
    where
        Self: Sized,
    {
        let mut builder = StreamBuilder::new(Some(vec![0]));
        self.to_html_async_with_buf::<true>(
            &mut builder,
            &mut Position::FirstChild,
        );
        builder.finish()
    }

    /// Renders a view to an HTML string, asynchronously.
    /* fn to_html_stream(self) -> impl Stream<Item = String>
    where
        Self: Sized,
    {
        use crate::ssr::handle_chunks;
        use futures::channel::mpsc::unbounded;

        let mut chunks = VecDeque::new();
        let mut curr = String::new();
        self.to_html_async_with_buf(
            &mut chunks,
            &mut curr,
            &PositionState::new(Position::FirstChild),
        );
        let (tx, rx) = unbounded();

        handle_chunks(tx, chunks).await;

        rx
    } */

    /// Renders a view to HTML, writing it into the given buffer.
    fn to_html_with_buf(self, buf: &mut String, position: &mut Position);

    /// Renders a view into a buffer of (synchronous or asynchronous) HTML chunks.
    fn to_html_async_with_buf<const OUT_OF_ORDER: bool>(
        self,
        buf: &mut StreamBuilder,
        position: &mut Position,
    ) where
        Self: Sized,
    {
        buf.with_buf(|buf| self.to_html_with_buf(buf, position));
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
        cursor: &Cursor<R>,
        position: &PositionState,
    ) -> Self::State;

    /// Hydrates using [`RenderHtml::hydrate`], beginning at the given element.
    fn hydrate_from<const FROM_SERVER: bool>(
        self,
        el: &R::Element,
    ) -> Self::State
    where
        Self: Sized,
    {
        self.hydrate_from_position::<FROM_SERVER>(el, Position::default())
    }

    /// Hydrates using [`RenderHtml::hydrate`], beginning at the given element and position.
    fn hydrate_from_position<const FROM_SERVER: bool>(
        self,
        el: &R::Element,
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
pub trait Mountable<R: Renderer> {
    /// Detaches the view from the DOM.
    fn unmount(&mut self);

    /// Mounts a node to the interface.
    fn mount(&mut self, parent: &R::Element, marker: Option<&R::Node>);

    /// Inserts another `Mountable` type before this one. Returns `false` if
    /// this does not actually exist in the UI (for example, `()`).
    fn insert_before_this(
        &self,
        parent: &R::Element,
        child: &mut dyn Mountable<R>,
    ) -> bool;

    /// Inserts another `Mountable` type before this one, or before the marker
    /// if this one doesn't exist in the UI (for example, `()`).
    fn insert_before_this_or_marker(
        &self,
        parent: &R::Element,
        child: &mut dyn Mountable<R>,
        marker: Option<&R::Node>,
    ) {
        if !self.insert_before_this(parent, child) {
            child.mount(parent, marker);
        }
    }
}

/// Indicates where a node should be mounted to its parent.
pub enum MountKind<R>
where
    R: Renderer,
{
    /// Node should be mounted before this marker node.
    Before(R::Node),
    /// Node should be appended to the parent’s children.
    Append,
}

impl<T, R> Mountable<R> for Option<T>
where
    T: Mountable<R>,
    R: Renderer,
{
    fn unmount(&mut self) {
        if let Some(ref mut mounted) = self {
            mounted.unmount()
        }
    }

    fn mount(&mut self, parent: &R::Element, marker: Option<&R::Node>) {
        if let Some(ref mut inner) = self {
            inner.mount(parent, marker);
        }
    }

    fn insert_before_this(
        &self,
        parent: &<R as Renderer>::Element,
        child: &mut dyn Mountable<R>,
    ) -> bool {
        self.as_ref()
            .map(|inner| inner.insert_before_this(parent, child))
            .unwrap_or(false)
    }
}

/// Allows data to be added to a static template.
pub trait ToTemplate {
    const TEMPLATE: &'static str = "";
    const CLASS: &'static str = "";
    const STYLE: &'static str = "";
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

#[derive(Debug, Default, Clone)]
pub struct PositionState(Arc<RwLock<Position>>);

impl PositionState {
    pub fn new(position: Position) -> Self {
        Self(Arc::new(RwLock::new(position)))
    }

    pub fn set(&self, position: Position) {
        *self.0.write() = position;
    }

    pub fn get(&self) -> Position {
        *self.0.read()
    }

    pub fn deep_clone(&self) -> Self {
        let current = self.get();
        Self(Arc::new(RwLock::new(current)))
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Default)]
pub enum Position {
    Current,
    #[default]
    FirstChild,
    NextChild,
    NextChildAfterText,
    OnlyChild,
    LastChild,
}
