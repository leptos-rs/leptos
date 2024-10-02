#![deny(missing_docs)]
#![forbid(unsafe_code)]

//! # Leptos Meta
//!
//! Leptos Meta allows you to modify content in a document’s `<head>` from within components
//! using the [`Leptos`](https://github.com/leptos-rs/leptos) web framework.
//!
//! Document metadata is updated automatically when running in the browser. For server-side
//! rendering, after the component tree is rendered to HTML, [`MetaContext::dehydrate`] can generate
//! HTML that should be injected into the `<head>` of the HTML document being rendered.
//!
//! ```
//! use leptos::prelude::*;
//! use leptos_meta::*;
//!
//! #[component]
//! fn MyApp() -> impl IntoView {
//!     // Provides a [`MetaContext`], if there is not already one provided.
//!     provide_meta_context();
//!
//!     let (name, set_name) = create_signal("Alice".to_string());
//!
//!     view! {
//!       <Title
//!         // reactively sets document.title when `name` changes
//!         text=move || name.get()
//!         // applies the `formatter` function to the `text` value
//!         formatter=|text| format!("“{text}” is your name")
//!       />
//!       <main>
//!         <input
//!           prop:value=move || name.get()
//!           on:input=move |ev| set_name.set(event_target_value(&ev))
//!         />
//!       </main>
//!     }
//! }
//! ```
//! # Feature Flags
//! - `csr` Client-side rendering: Generate DOM nodes in the browser
//! - `ssr` Server-side rendering: Generate an HTML string (typically on the server)
//! - `hydrate` Hydration: use this to add interactivity to an SSRed Leptos app
//! - `stable` By default, Leptos requires `nightly` Rust, which is what allows the ergonomics
//!   of calling signals as functions. Enable this feature to support `stable` Rust.
//!
//! **Important Note:** You must enable one of `csr`, `hydrate`, or `ssr` to tell Leptos
//! which mode your app is operating in.

use futures::{Stream, StreamExt};
use leptos::{
    attr::NextAttribute,
    component,
    logging::debug_warn,
    reactive::owner::{provide_context, use_context},
    tachys::{
        dom::document,
        html::{
            attribute::Attribute,
            element::{ElementType, HtmlElement},
        },
        hydration::Cursor,
        view::{
            add_attr::AddAnyAttr, Mountable, Position, PositionState, Render,
            RenderHtml,
        },
    },
    IntoView,
};
use once_cell::sync::Lazy;
use send_wrapper::SendWrapper;
use std::{
    fmt::Debug,
    sync::{
        mpsc::{channel, Receiver, Sender},
        Arc,
    },
};
use wasm_bindgen::JsCast;
use web_sys::HtmlHeadElement;

mod body;
mod html;
mod link;
mod meta_tags;
mod script;
mod style;
mod stylesheet;
mod title;
pub use body::*;
pub use html::*;
pub use link::*;
pub use meta_tags::*;
pub use script::*;
pub use style::*;
pub use stylesheet::*;
pub use title::*;

/// Contains the current state of meta tags. To access it, you can use [`use_head`].
///
/// This should generally by provided somewhere in the root of your application using
/// [`provide_meta_context`].
#[derive(Clone, Debug)]
pub struct MetaContext {
    /// Metadata associated with the `<title>` element.
    pub(crate) title: TitleContext,
    /// The hydration cursor for the location in the `<head>` for arbitrary tags will be rendered.
    pub(crate) cursor: Arc<Lazy<SendWrapper<Cursor>>>,
}

impl MetaContext {
    /// Creates an empty [`MetaContext`].
    pub fn new() -> Self {
        Default::default()
    }
}

pub(crate) const HEAD_MARKER_COMMENT: &str = "HEAD";
/// Return value of [`Node::node_type`] for a comment.
/// https://developer.mozilla.org/en-US/docs/Web/API/Node/nodeType#node.comment_node
const COMMENT_NODE: u16 = 8;

impl Default for MetaContext {
    fn default() -> Self {
        let build_cursor: fn() -> SendWrapper<Cursor> = || {
            let head = document().head().expect("missing <head> element");
            let mut cursor = None;
            let mut child = head.first_child();
            while let Some(this_child) = child {
                if this_child.node_type() == COMMENT_NODE
                    && this_child.text_content().as_deref()
                        == Some(HEAD_MARKER_COMMENT)
                {
                    cursor = Some(this_child);
                    break;
                }
                child = this_child.next_sibling();
            }
            SendWrapper::new(Cursor::new(
                cursor
                    .expect(
                        "no leptos_meta HEAD marker comment found. Did you \
                         include the <MetaTags/> component in the <head> of \
                         your server-rendered app?",
                    )
                    .unchecked_into(),
            ))
        };

        let cursor = Arc::new(Lazy::new(build_cursor));
        Self {
            title: Default::default(),
            cursor,
        }
    }
}

/// Allows you to add `<head>` content from components located in the `<body>` of the application,
/// which can be accessed during server rendering via [`ServerMetaContextOutput`].
///
/// This should be provided as context during server rendering.
///
/// No content added after the first chunk of the stream has been sent will be included in the
/// initial `<head>`. Data that needs to be included in the `<head>` during SSR should be
/// synchronous or loaded as a blocking resource.
#[derive(Clone, Debug)]
pub struct ServerMetaContext {
    /// Metadata associated with the `<title>` element.
    pub(crate) title: TitleContext,
    /// Attributes for the `<html>` element.
    pub(crate) html: Sender<String>,
    /// Attributes for the `<body>` element.
    pub(crate) body: Sender<String>,
    /// Arbitrary elements to be added to the `<head>` as HTML.
    #[allow(unused)] // used in SSR
    pub(crate) elements: Sender<String>,
}

/// Allows you to access `<head>` content that was inserted via [`ServerMetaContext`].
#[must_use = "If you do not use the output, adding meta tags will have no \
              effect."]
#[derive(Debug)]
pub struct ServerMetaContextOutput {
    pub(crate) title: TitleContext,
    html: Receiver<String>,
    body: Receiver<String>,
    elements: Receiver<String>,
}

impl ServerMetaContext {
    /// Creates an empty [`ServerMetaContext`].
    pub fn new() -> (ServerMetaContext, ServerMetaContextOutput) {
        let title = TitleContext::default();
        let (html_tx, html_rx) = channel();
        let (body_tx, body_rx) = channel();
        let (elements_tx, elements_rx) = channel();
        let tx = ServerMetaContext {
            title: title.clone(),
            html: html_tx,
            body: body_tx,
            elements: elements_tx,
        };
        let rx = ServerMetaContextOutput {
            title,
            html: html_rx,
            body: body_rx,
            elements: elements_rx,
        };
        (tx, rx)
    }
}

impl ServerMetaContextOutput {
    /// Consumes the metadata, injecting it into the the first chunk of an HTML stream in the
    /// appropriate place.
    ///
    /// This means that only meta tags rendered during the first chunk of the stream will be
    /// included.
    pub async fn inject_meta_context(
        self,
        mut stream: impl Stream<Item = String> + Send + Unpin,
    ) -> impl Stream<Item = String> + Send {
        // wait for the first chunk of the stream, to ensure our components hve run
        let mut first_chunk = stream.next().await.unwrap_or_default();

        // create <title> tag
        let title = self.title.as_string();
        let title_len = title
            .as_ref()
            .map(|n| "<title>".len() + n.len() + "</title>".len())
            .unwrap_or(0);

        // collect all registered meta tags
        let meta_buf = self.elements.try_iter().collect::<String>();

        // get HTML strings for `<html>` and `<body>`
        let html_attrs = self.html.try_iter().collect::<String>();
        let body_attrs = self.body.try_iter().collect::<String>();

        let mut modified_chunk = if title_len == 0 && meta_buf.is_empty() {
            first_chunk
        } else {
            let mut buf = String::with_capacity(
                first_chunk.len() + title_len + meta_buf.len(),
            );
            let head_loc = first_chunk
                .find("</head>")
                .expect("you are using leptos_meta without a </head> tag");
            let marker_loc =
                first_chunk.find("<!--HEAD-->").unwrap_or_else(|| {
                    first_chunk.find("</head>").unwrap_or(head_loc)
                });
            let (before_marker, after_marker) =
                first_chunk.split_at_mut(marker_loc);
            let (before_head_close, after_head) =
                after_marker.split_at_mut(head_loc - marker_loc);
            buf.push_str(before_marker);
            if let Some(title) = title {
                buf.push_str("<title>");
                buf.push_str(&title);
                buf.push_str("</title>");
            }
            buf.push_str(before_head_close);
            buf.push_str(&meta_buf);
            buf.push_str(after_head);
            buf
        };

        if !html_attrs.is_empty() {
            if let Some(index) = modified_chunk.find("<html") {
                // Calculate the position where the new string should be inserted
                let insert_pos = index + "<html".len();
                modified_chunk.insert_str(insert_pos, &html_attrs);
            }
        }

        if !body_attrs.is_empty() {
            if let Some(index) = modified_chunk.find("<body") {
                // Calculate the position where the new string should be inserted
                let insert_pos = index + "<body".len();
                modified_chunk.insert_str(insert_pos, &body_attrs);
            }
        }

        futures::stream::once(async move { modified_chunk }).chain(stream)
    }
}

/// Provides a [`MetaContext`], if there is not already one provided. This ensures that you can provide it
/// at the highest possible level, without overwriting a [`MetaContext`] that has already been provided
/// (for example, by a server-rendering integration.)
pub fn provide_meta_context() {
    if use_context::<MetaContext>().is_none() {
        provide_context(MetaContext::new());
    }
}

/// Returns the current [`MetaContext`].
///
/// If there is no [`MetaContext`] in this or any parent scope, this will
/// create a new [`MetaContext`] and provide it to the current scope.
///
/// Note that this may cause confusing behavior, e.g., if multiple nested routes independently
/// call `use_head()` but a single [`MetaContext`] has not been provided at the application root.
/// The best practice is always to call [`provide_meta_context`] early in the application.
pub fn use_head() -> MetaContext {
    match use_context::<MetaContext>() {
        None => {
            debug_warn!(
                "use_head() is being called without a MetaContext being \
                 provided. We'll automatically create and provide one, but if \
                 this is being called in a child route it may cause bugs. To \
                 be safe, you should provide_meta_context() somewhere in the \
                 root of the app."
            );
            let meta = MetaContext::new();
            provide_context(meta.clone());
            meta
        }
        Some(ctx) => ctx,
    }
}

pub(crate) fn register<E, At, Ch>(
    el: HtmlElement<E, At, Ch>,
) -> RegisteredMetaTag<E, At, Ch>
where
    HtmlElement<E, At, Ch>: RenderHtml,
{
    #[allow(unused_mut)] // used for `ssr`
    let mut el = Some(el);

    #[cfg(feature = "ssr")]
    if let Some(cx) = use_context::<ServerMetaContext>() {
        let mut buf = String::new();
        el.take().unwrap().to_html_with_buf(
            &mut buf,
            &mut Position::NextChild,
            false,
            false,
        );
        _ = cx.elements.send(buf); // fails only if the receiver is already dropped
    } else {
        let msg = "tried to use a leptos_meta component without \
                   `ServerMetaContext` provided";

        #[cfg(feature = "tracing")]
        tracing::warn!("{}", msg);

        #[cfg(not(feature = "tracing"))]
        eprintln!("{}", msg);
    }

    RegisteredMetaTag { el }
}

struct RegisteredMetaTag<E, At, Ch> {
    // this is `None` if we've already taken it out to render to HTML on the server
    // we don't render it in place in RenderHtml, so it's fine
    el: Option<HtmlElement<E, At, Ch>>,
}

struct RegisteredMetaTagState<E, At, Ch>
where
    HtmlElement<E, At, Ch>: Render,
{
    state: <HtmlElement<E, At, Ch> as Render>::State,
}

impl<E, At, Ch> Drop for RegisteredMetaTagState<E, At, Ch>
where
    HtmlElement<E, At, Ch>: Render,
{
    fn drop(&mut self) {
        self.state.unmount();
    }
}

fn document_head() -> HtmlHeadElement {
    let document = document();
    document.head().unwrap_or_else(|| {
        let el = document.create_element("head").unwrap();
        let document = document.document_element().unwrap();
        _ = document.append_child(&el);
        el.unchecked_into()
    })
}

impl<E, At, Ch> Render for RegisteredMetaTag<E, At, Ch>
where
    E: ElementType,
    At: Attribute,
    Ch: Render,
{
    type State = RegisteredMetaTagState<E, At, Ch>;

    fn build(self) -> Self::State {
        let state = self.el.unwrap().build();
        RegisteredMetaTagState { state }
    }

    fn rebuild(self, state: &mut Self::State) {
        self.el.unwrap().rebuild(&mut state.state);
    }
}

impl<E, At, Ch> AddAnyAttr for RegisteredMetaTag<E, At, Ch>
where
    E: ElementType + Send,
    At: Attribute + Send,
    Ch: RenderHtml + Send,
{
    type Output<SomeNewAttr: Attribute> =
        RegisteredMetaTag<E, <At as NextAttribute>::Output<SomeNewAttr>, Ch>;

    fn add_any_attr<NewAttr: Attribute>(
        self,
        attr: NewAttr,
    ) -> Self::Output<NewAttr>
    where
        Self::Output<NewAttr>: RenderHtml,
    {
        RegisteredMetaTag {
            el: self.el.map(|inner| inner.add_any_attr(attr)),
        }
    }
}

impl<E, At, Ch> RenderHtml for RegisteredMetaTag<E, At, Ch>
where
    E: ElementType,
    At: Attribute,
    Ch: RenderHtml + Send,
{
    type AsyncOutput = Self;

    const MIN_LENGTH: usize = 0;

    fn dry_resolve(&mut self) {
        self.el.dry_resolve()
    }

    async fn resolve(self) -> Self::AsyncOutput {
        self // TODO?
    }

    fn to_html_with_buf(
        self,
        _buf: &mut String,
        _position: &mut Position,
        _escape: bool,
        _mark_branches: bool,
    ) {
        // meta tags are rendered into the buffer stored into the context
        // the value has already been taken out, when we're on the server
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        _cursor: &Cursor,
        _position: &PositionState,
    ) -> Self::State {
        let cursor = use_context::<MetaContext>()
            .expect(
                "attempting to hydrate `leptos_meta` components without a \
                 MetaContext provided",
            )
            .cursor;
        let state = self.el.unwrap().hydrate::<FROM_SERVER>(
            &cursor,
            &PositionState::new(Position::NextChild),
        );
        RegisteredMetaTagState { state }
    }
}

impl<E, At, Ch> Mountable for RegisteredMetaTagState<E, At, Ch>
where
    E: ElementType,
    At: Attribute,
    Ch: Render,
{
    fn unmount(&mut self) {
        self.state.unmount();
    }

    fn mount(
        &mut self,
        _parent: &leptos::tachys::renderer::types::Element,
        _marker: Option<&leptos::tachys::renderer::types::Node>,
    ) {
        // we always mount this to the <head>, which is the whole point
        // but this shouldn't warn about the parent being a regular element or being unused
        // because it will call "mount" with the parent where it is located in the component tree,
        // but actually be mounted to the <head>
        self.state.mount(&document_head(), None);
    }

    fn insert_before_this(&self, _child: &mut dyn Mountable) -> bool {
        // Registered meta tags will be mounted in the <head>, but *seem* to be mounted somewhere
        // else in the DOM. We should never tell the renderer that we have successfully mounted
        // something before this, because if e.g., a <Meta/> is the first item in an Either, then
        // the alternate view will end up being mounted in the <head> -- which is not at all what
        // we intended!
        false
    }
}

/// During server rendering, inserts the meta tags that have been generated by the other components
/// in this crate into the DOM. This should be placed somewhere inside the `<head>` element that is
/// being used during server rendering.
#[component]
pub fn MetaTags() -> impl IntoView {
    MetaTagsView
}

#[derive(Debug)]
struct MetaTagsView;

// this implementation doesn't do anything during client-side rendering, it's just for server-side
// rendering HTML for all the tags that will be injected into the `<head>`
//
// client-side rendering is handled by the individual components
impl Render for MetaTagsView {
    type State = ();

    fn build(self) -> Self::State {}

    fn rebuild(self, _state: &mut Self::State) {}
}

impl AddAnyAttr for MetaTagsView {
    type Output<SomeNewAttr: Attribute> = MetaTagsView;

    fn add_any_attr<NewAttr: Attribute>(
        self,
        _attr: NewAttr,
    ) -> Self::Output<NewAttr>
    where
        Self::Output<NewAttr>: RenderHtml,
    {
        self
    }
}

impl RenderHtml for MetaTagsView {
    type AsyncOutput = Self;

    const MIN_LENGTH: usize = 0;

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
        buf.push_str("<!--HEAD-->");
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        _cursor: &Cursor,
        _position: &PositionState,
    ) -> Self::State {
    }
}
