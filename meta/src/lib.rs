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
//! use leptos::*;
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

use indexmap::IndexMap;
use leptos::{
    component, debug_warn,
    reactive_graph::owner::{provide_context, use_context},
    tachys::{
        dom::document,
        html::{
            attribute::{any_attribute::AnyAttribute, Attribute},
            element::{CreateElement, ElementType, HtmlElement},
        },
        hydration::Cursor,
        renderer::{dom::Dom, Renderer},
        view::{Mountable, Position, PositionState, Render, RenderHtml},
    },
    IntoView,
};
use once_cell::sync::Lazy;
use or_poisoned::OrPoisoned;
use send_wrapper::SendWrapper;
use std::{
    cell::{Cell, RefCell},
    fmt::Debug,
    rc::Rc,
    sync::{Arc, RwLock},
};
use wasm_bindgen::JsCast;
use web_sys::{HtmlHeadElement, Node};

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
    pub(crate) cursor: Arc<Lazy<SendWrapper<Cursor<Dom>>>>,
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
        let build_cursor: fn() -> SendWrapper<Cursor<Dom>> = || {
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
                    .expect("no leptos_meta HEAD marker comment found")
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

/// Contains the state of meta tags for server rendering.
///
/// This should be provided as context during server rendering.
#[derive(Clone, Default)]
pub struct ServerMetaContext {
    inner: Arc<RwLock<ServerMetaContextInner>>,
    /// Metadata associated with the `<title>` element.
    pub(crate) title: TitleContext,
}

#[derive(Default)]
struct ServerMetaContextInner {
    /*/// Metadata associated with the `<html>` element
    pub html: HtmlContext,
    /// Metadata associated with the `<title>` element.
    pub title: TitleContext,*/
    /// Metadata associated with the `<html>` element
    pub(crate) html: Vec<AnyAttribute<Dom>>,
    /// Metadata associated with the `<body>` element
    pub(crate) body: Vec<AnyAttribute<Dom>>,
    /// HTML for arbitrary tags that will be included in the `<head>` element
    pub(crate) head_html: String,
}

impl Debug for ServerMetaContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ServerMetaContext").finish_non_exhaustive()
    }
}

impl ServerMetaContext {
    /// Creates an empty [`ServerMetaContext`].
    pub fn new() -> Self {
        Default::default()
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
    el: HtmlElement<E, At, Ch, Dom>,
) -> RegisteredMetaTag<E, At, Ch>
where
    HtmlElement<E, At, Ch, Dom>: RenderHtml<Dom>,
{
    let mut el = Some(el);

    if let Some(cx) = use_context::<ServerMetaContext>() {
        let mut inner = cx.inner.write().or_poisoned();
        el.take()
            .unwrap()
            .to_html_with_buf(&mut inner.head_html, &mut Position::NextChild);
    }

    RegisteredMetaTag { el }
}

struct RegisteredMetaTag<E, At, Ch> {
    // this is `None` if we've already taken it out to render to HTML on the server
    // we don't render it in place in RenderHtml, so it's fine
    el: Option<HtmlElement<E, At, Ch, Dom>>,
}

struct RegisteredMetaTagState<E, At, Ch>
where
    HtmlElement<E, At, Ch, Dom>: Render<Dom>,
{
    state: <HtmlElement<E, At, Ch, Dom> as Render<Dom>>::State,
}

fn document_head() -> HtmlHeadElement {
    let document = document();
    document.head().unwrap_or_else(|| {
        let el = document.create_element("head").unwrap();
        let document = document.document_element().unwrap();
        document.append_child(&el);
        el.unchecked_into()
    })
}

impl<E, At, Ch> Render<Dom> for RegisteredMetaTag<E, At, Ch>
where
    E: CreateElement<Dom>,
    At: Attribute<Dom>,
    Ch: Render<Dom>,
{
    type State = RegisteredMetaTagState<E, At, Ch>;
    type FallibleState = RegisteredMetaTagState<E, At, Ch>;

    fn build(self) -> Self::State {
        let state = self.el.unwrap().build();
        RegisteredMetaTagState { state }
    }

    fn rebuild(self, state: &mut Self::State) {
        self.el.unwrap().rebuild(&mut state.state);
    }

    fn try_build(self) -> leptos::tachys::error::Result<Self::FallibleState> {
        Ok(self.build())
    }

    fn try_rebuild(
        self,
        state: &mut Self::FallibleState,
    ) -> leptos::tachys::error::Result<()> {
        self.rebuild(state);
        Ok(())
    }
}

impl<E, At, Ch> RenderHtml<Dom> for RegisteredMetaTag<E, At, Ch>
where
    E: ElementType + CreateElement<Dom>,
    At: Attribute<Dom>,
    Ch: RenderHtml<Dom>,
{
    const MIN_LENGTH: usize = 0;

    fn to_html_with_buf(self, _buf: &mut String, _position: &mut Position) {
        // meta tags are rendered into the buffer stored into the context
        // the value has already been taken out, when we're on the server
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        cursor: &Cursor<Dom>,
        position: &PositionState,
    ) -> Self::State {
        let cursor = use_context::<MetaContext>()
            .expect(
                "attempting to hydrate `leptos_meta` components without a \
                 MetaContext provided",
            )
            .cursor;
        let state = self.el.unwrap().hydrate::<FROM_SERVER>(
            &*cursor,
            &PositionState::new(Position::NextChild),
        );
        RegisteredMetaTagState { state }
    }
}

impl<E, At, Ch> Mountable<Dom> for RegisteredMetaTagState<E, At, Ch>
where
    E: CreateElement<Dom>,
    At: Attribute<Dom>,
    Ch: Render<Dom>,
{
    fn unmount(&mut self) {
        self.state.unmount();
    }

    fn mount(
        &mut self,
        _parent: &<Dom as Renderer>::Element,
        _marker: Option<&<Dom as Renderer>::Node>,
    ) {
        // we always mount this to the <head>, which is the whole point
        // but this shouldn't warn about the parent being a regular element or being unused
        // because it will call "mount" with the parent where it is located in the component tree,
        // but actually be mounted to the <head>
        self.state.mount(&document_head(), None);
    }

    fn insert_before_this(
        &self,
        parent: &<Dom as Renderer>::Element,
        child: &mut dyn Mountable<Dom>,
    ) -> bool {
        self.state.insert_before_this(&document_head(), child)
    }
}

/// During server rendering, inserts the meta tags that have been generated by the other components
/// in this crate into the DOM. This should be placed somewhere inside the `<head>` element that is
/// being used during server rendering.
#[component]
pub fn MetaTags() -> impl IntoView {
    MetaTagsView {
        context: use_context::<ServerMetaContext>().expect(
            "before using the <MetaTags/> component, you should make sure to \
             provide ServerMetaContext via context",
        ),
    }
}

struct MetaTagsView {
    context: ServerMetaContext,
}

// this implementation doesn't do anything during client-side rendering, it's just for server-side
// rendering HTML for all the tags that will be injected into the `<head>`
//
// client-side rendering is handled by the individual components
impl Render<Dom> for MetaTagsView {
    type State = ();
    type FallibleState = ();

    fn build(self) -> Self::State {}

    fn rebuild(self, state: &mut Self::State) {}

    fn try_build(self) -> leptos::tachys::error::Result<Self::FallibleState> {
        Ok(())
    }

    fn try_rebuild(
        self,
        state: &mut Self::FallibleState,
    ) -> leptos::tachys::error::Result<()> {
        Ok(())
    }
}

impl RenderHtml<Dom> for MetaTagsView {
    const MIN_LENGTH: usize = 0;

    fn to_html_with_buf(self, buf: &mut String, position: &mut Position) {
        if let Some(title) = self.context.title.as_string() {
            buf.reserve(15 + title.len());
            buf.push_str("<title>");
            buf.push_str(&title);
            buf.push_str("</title>");
        }

        buf.push_str(&self.context.inner.write().or_poisoned().head_html);
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        cursor: &Cursor<Dom>,
        position: &PositionState,
    ) -> Self::State {
    }
}

impl MetaContext {
    // TODO remove the below?

    #[cfg(feature = "ssr")]
    /// Converts the existing metadata tags into HTML that can be injected into the document head.
    ///
    /// This should be called *after* the app’s component tree has been rendered into HTML, so that
    /// components can set meta tags.
    ///
    /// ```
    /// use leptos::*;
    /// use leptos_meta::*;
    ///
    /// # #[cfg(not(any(feature = "csr", feature = "hydrate")))] {
    /// # let runtime = create_runtime();
    ///   provide_meta_context();
    ///
    ///   let app = view! {
    ///     <main>
    ///       <Title text="my title"/>
    ///       <Stylesheet href="/style.css"/>
    ///       <p>"Some text"</p>
    ///     </main>
    ///   };
    ///
    ///   // `app` contains only the body content w/ hydration stuff, not the meta tags
    ///   assert!(
    ///      !app.into_view().render_to_string().contains("my title")
    ///   );
    ///   // `MetaContext::dehydrate()` gives you HTML that should be in the `<head>`
    ///   assert!(use_head().dehydrate().contains("<title>my title</title>"));
    /// # runtime.dispose();
    /// # }
    /// ```
    pub fn dehydrate(&self) -> String {
        let mut tags = String::new();

        // Title
        if let Some(title) = self.title.as_string() {
            tags.push_str("<title>");
            tags.push_str(&title);
            tags.push_str("</title>");
        }
        tags.push_str(&self.tags.as_string());

        tags
    }
}

/// Extracts the metadata that should be used to close the `<head>` tag
/// and open the `<body>` tag. This is a helper function used in implementing
/// server-side HTML rendering across crates.
#[cfg(feature = "ssr")]
pub fn generate_head_metadata() -> String {
    let (head, body) = generate_head_metadata_separated();
    format!("{head}</head>{body}")
}

/// Extracts the metadata that should be inserted at the beginning of the `<head>` tag
/// and on the opening `<body>` tag. This is a helper function used in implementing
/// server-side HTML rendering across crates.
#[cfg(feature = "ssr")]
pub fn generate_head_metadata_separated() -> (String, String) {
    let meta = use_context::<MetaContext>();
    let head = meta
        .as_ref()
        .map(|meta| meta.dehydrate())
        .unwrap_or_default();
    let body_meta = meta
        .as_ref()
        .and_then(|meta| meta.body.as_string())
        .unwrap_or_default();
    (head, format!("<body{body_meta}>"))
}
