#![deny(missing_docs)]
#![forbid(unsafe_code)]

//! # Leptos Meta
//!
//! Leptos Meta allows you to modify content in a document’s `<head>` from within components
//! using the [Leptos](https://github.com/leptos-rs/leptos) web framework.
//!
//! Document metadata is updated automatically when running in the browser. For server-side
//! rendering, after the component tree is rendered to HTML, [MetaContext::dehydrate] can generate
//! HTML that should be injected into the `<head>` of the HTML document being rendered.
//!
//! ```
//! use leptos::*;
//! use leptos_meta::*;
//!
//! #[component]
//! fn MyApp(cx: Scope) -> impl IntoView {
//!     let (name, set_name) = create_signal(cx, "Alice".to_string());
//!
//!     view! { cx,
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

use cfg_if::cfg_if;
use indexmap::IndexMap;
use leptos::{
    leptos_dom::{debug_warn, html::AnyElement},
    *,
};
use std::{
    borrow::Cow,
    cell::{Cell, RefCell},
    fmt::Debug,
    rc::Rc,
};
#[cfg(any(feature = "csr", feature = "hydrate"))]
use wasm_bindgen::{JsCast, UnwrapThrowExt};

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

/// Contains the current state of meta tags. To access it, you can use [use_head].
///
/// This should generally by provided somewhere in the root of your application using
/// [provide_meta_context].
#[derive(Clone, Default, Debug)]
pub struct MetaContext {
    /// Metadata associated with the `<html>` element
    pub html: HtmlContext,
    /// Metadata associated with the `<title>` element.
    pub title: TitleContext,
    /// Metadata associated with the `<body>` element
    pub body: BodyContext,
    /// Other metadata tags.
    pub tags: MetaTagsContext,
}

/// Manages all of the element created by components.
#[derive(Clone, Default)]
pub struct MetaTagsContext {
    next_id: Rc<Cell<MetaTagId>>,
    #[allow(clippy::type_complexity)]
    els: Rc<
        RefCell<
            IndexMap<
                Cow<'static, str>,
                (HtmlElement<AnyElement>, Scope, Option<web_sys::Element>),
            >,
        >,
    >,
}

impl std::fmt::Debug for MetaTagsContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MetaTagsContext").finish()
    }
}

impl MetaTagsContext {
    /// Converts metadata tags into an HTML string.
    #[cfg(any(feature = "ssr", docs))]
    pub fn as_string(&self) -> String {
        self.els
            .borrow()
            .iter()
            .map(|(_, (builder_el, cx, _))| {
                builder_el.clone().into_view(*cx).render_to_string(*cx)
            })
            .collect()
    }

    #[doc(hidden)]
    pub fn register(
        &self,
        cx: Scope,
        id: Cow<'static, str>,
        builder_el: HtmlElement<AnyElement>,
    ) {
        cfg_if! {
            if #[cfg(any(feature = "csr", feature = "hydrate"))] {
                use leptos::document;

                let element_to_hydrate = document()
                    .get_element_by_id(&id);

                let el = element_to_hydrate.unwrap_or_else({
                    let builder_el = builder_el.clone();
                    move || {
                        let head = document().head().unwrap_throw();
                        head
                            .append_child(&builder_el)
                            .unwrap_throw();

                        (*builder_el).clone().unchecked_into()
                    }
                });

                on_cleanup(cx, {
                    let el = el.clone();
                    let els = self.els.clone();
                    let id = id.clone();
                    move || {
                        let head = document().head().unwrap_throw();
                        _ = head.remove_child(&el);
                        els.borrow_mut().remove(&id);
                    }
                });

                self
                    .els
                    .borrow_mut()
                    .insert(id, (builder_el.into_any(), cx, Some(el)));

            } else {
                self.els.borrow_mut().insert(id, (builder_el, cx, None));
            }
        }
    }
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
struct MetaTagId(usize);

impl MetaTagsContext {
    fn get_next_id(&self) -> MetaTagId {
        let current_id = self.next_id.get();
        let next_id = MetaTagId(current_id.0 + 1);
        self.next_id.set(next_id);
        next_id
    }
}

/// Provides a [MetaContext], if there is not already one provided. This ensures that you can provide it
/// at the highest possible level, without overwriting a [MetaContext] that has already been provided
/// (for example, by a server-rendering integration.)
pub fn provide_meta_context(cx: Scope) {
    if use_context::<MetaContext>(cx).is_none() {
        provide_context(cx, MetaContext::new());
    }
}

/// Returns the current [MetaContext].
///
/// If there is no [MetaContext] in this [Scope](leptos::Scope) or any parent scope, this will
/// create a new [MetaContext] and provide it to the current scope.
///
/// Note that this may cause confusing behavior, e.g., if multiple nested routes independently
/// call `use_head()` but a single [MetaContext] has not been provided at the application root.
/// The best practice is always to call [provide_meta_context] early in the application.
pub fn use_head(cx: Scope) -> MetaContext {
    #[cfg(debug_assertions)]
    feature_warning();

    match use_context::<MetaContext>(cx) {
        None => {
            debug_warn!(
                "use_head() is being called without a MetaContext being \
                 provided. We'll automatically create and provide one, but if \
                 this is being called in a child route it may cause bugs. To \
                 be safe, you should provide_meta_context(cx) somewhere in \
                 the root of the app."
            );
            let meta = MetaContext::new();
            provide_context(cx, meta.clone());
            meta
        }
        Some(ctx) => ctx,
    }
}

impl MetaContext {
    /// Creates an empty [MetaContext].
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
    /// run_scope(create_runtime(), |cx| {
    ///   provide_meta_context(cx);
    ///
    ///   let app = view! { cx,
    ///     <main>
    ///       <Title text="my title"/>
    ///       <Stylesheet href="/style.css"/>
    ///       <p>"Some text"</p>
    ///     </main>
    ///   };
    ///
    ///   // `app` contains only the body content w/ hydration stuff, not the meta tags
    ///   assert!(
    ///      !app.into_view(cx).render_to_string(cx).contains("my title")
    ///   );
    ///   // `MetaContext::dehydrate()` gives you HTML that should be in the `<head>`
    ///   assert!(use_head(cx).dehydrate().contains("<title>my title</title>"))
    /// });
    /// # }
    /// ```
    pub fn dehydrate(&self) -> String {
        use leptos::leptos_dom::HydrationCtx;

        let prev_key = HydrationCtx::peek();
        let mut tags = String::new();

        // Title
        if let Some(title) = self.title.as_string() {
            tags.push_str("<title>");
            tags.push_str(&title);
            tags.push_str("</title>");
        }
        tags.push_str(&self.tags.as_string());

        HydrationCtx::continue_from(prev_key);
        tags
    }
}

/// Extracts the metadata that should be used to close the `<head>` tag
/// and open the `<body>` tag. This is a helper function used in implementing
/// server-side HTML rendering across crates.
#[cfg(feature = "ssr")]
pub fn generate_head_metadata(cx: Scope) -> String {
    let (head, body) = generate_head_metadata_separated(cx);
    format!("{head}</head><{body}>")
}

/// Extracts the metadata that should be inserted at the beginning of the `<head>` tag
/// and on the opening `<body>` tag. This is a helper function used in implementing
/// server-side HTML rendering across crates.
#[cfg(feature = "ssr")]
pub fn generate_head_metadata_separated(cx: Scope) -> (String, String) {
    let meta = use_context::<MetaContext>(cx);
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

#[cfg(debug_assertions)]
pub(crate) fn feature_warning() {
    if !cfg!(any(feature = "csr", feature = "hydrate", feature = "ssr")) {
        leptos::debug_warn!("WARNING: `leptos_meta` does nothing unless you enable one of its features (`csr`, `hydrate`, or `ssr`). See the docs at https://docs.rs/leptos_meta/latest/leptos_meta/ for more information.");
    }
}
