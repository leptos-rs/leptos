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
    debug_warn,
    reactive_graph::owner::{provide_context, use_context},
    tachys::{
        html::attribute::any_attribute::AnyAttribute, renderer::dom::Dom,
    },
};
use std::{
    cell::{Cell, RefCell},
    fmt::Debug,
    rc::Rc,
    sync::{Arc, RwLock},
};
#[cfg(any(feature = "csr", feature = "hydrate"))]
use wasm_bindgen::{JsCast, UnwrapThrowExt};

mod body;
mod html;
/*mod link;
mod meta_tags;
mod script;
mod style;
mod stylesheet;*/
mod title;
pub use body::*;
pub use html::*;
/*pub use link::*;
pub use meta_tags::*;
pub use script::*;
pub use style::*;
pub use stylesheet::*;*/
pub use title::*;

/// Contains the current state of meta tags. To access it, you can use [`use_head`].
///
/// This should generally by provided somewhere in the root of your application using
/// [`provide_meta_context`].
#[derive(Clone, Default, Debug)]
pub struct MetaContext {
    /// Metadata associated with the `<title>` element.
    pub title: TitleContext,
    /*
    /// Other metadata tags.
    pub tags: MetaTagsContext,
    */
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
    /*
    /// Other metadata tags.
    pub tags: MetaTagsContext,
    */
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

impl MetaContext {
    /// Creates an empty [`MetaContext`].
    pub fn new() -> Self {
        Default::default()
    }

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
