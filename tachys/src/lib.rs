//! Allows rendering user interfaces based on a statically-typed view tree.
//!
//! This view tree is generic over rendering backends, and agnostic about reactivity/change
//! detection.

// this is specifically used for `unsized_const_params` below
// this allows us to use const generic &'static str for static text nodes and attributes
#![allow(incomplete_features)]
#![cfg_attr(
    all(feature = "nightly", rustc_nightly),
    feature(unsized_const_params)
)]
// support for const generic &'static str has now moved back and forth between
// these two features a couple times; we'll just enable both
#![cfg_attr(all(feature = "nightly", rustc_nightly), feature(adt_const_params))]
#![deny(missing_docs)]

/// Commonly-used traits.
pub mod prelude {
    #[cfg(feature = "web")]
    pub use crate::{
        html::{
            attribute::{
                any_attribute::IntoAnyAttribute,
                aria::AriaAttributes,
                custom::CustomAttribute,
                global::{
                    ClassAttribute, GlobalAttributes, GlobalOnAttributes,
                    OnAttribute, OnTargetAttribute, PropAttribute,
                    StyleAttribute,
                },
                IntoAttributeValue,
            },
            directive::DirectiveAttribute,
            element::{ElementChild, ElementExt, InnerHtmlAttribute},
            node_ref::NodeRefAttribute,
        },
        renderer::dom::Dom,
    };

    pub use crate::{
        renderer::Renderer,
        view::{
            add_attr::AddAnyAttr,
            any_view::{AnyView, IntoAny, IntoMaybeErased},
            IntoRender, Mountable, Render, RenderHtml,
        },
    };

    // Native: re-export the cocoa renderer alias as `Dom` so existing
    // `use prelude::Dom` sites keep resolving on macOS. Only active
    // when the `native-ui` feature is enabled (otherwise the cocoa
    // module isn't compiled).
    #[cfg(all(target_os = "macos", feature = "native-ui"))]
    pub use crate::renderer::cocoa::Dom;
    // Same for iOS — the UIKit renderer alias is also `Dom`.
    #[cfg(all(target_os = "ios", feature = "native-ui"))]
    pub use crate::renderer::ios::Dom;
    // Same for Linux — the GTK renderer alias is also `Dom`.
    #[cfg(all(target_os = "linux", feature = "native-ui"))]
    pub use crate::renderer::gtk::Dom;

    #[cfg(feature = "native-ui")]
    pub use crate::html::attribute::{
        any_attribute::IntoAnyAttribute, IntoAttributeValue,
    };
}

#[cfg(feature = "web")]
use wasm_bindgen::JsValue;
#[cfg(feature = "web")]
use web_sys::Node;

// Per-OS element builder modules (`cocoa`, `ios`, `gtk`) moved to
// the per-renderer glue crates `leptos_cocoa` / `leptos_ios` /
// `leptos_gtk` in Phase 5. Tachys keeps only the renderer-protocol
// adapter in `renderer::{cocoa,ios,gtk}` (used by the `Rndr` type
// alias for tachys's generic machinery).
/// Helpers for interacting with the DOM (web only).
#[cfg(feature = "web")]
pub mod dom;
/// Types for building a statically-typed HTML view tree.
pub mod html;
/// Supports adding interactivity to HTML. The bulk of this module
/// is web-only (DOM cursor traversal), but a handful of native-side
/// stubs and the `Cursor` type are referenced by the trait surface
/// (`RenderHtml`), so the module compiles unconditionally — its
/// web-only contents are gated internally on `feature = "web"`.
pub mod hydration;
/// Types for MathML (web only).
#[cfg(feature = "web")]
pub mod mathml;
/// Defines various backends that can render views.
pub mod renderer;
/// Rendering views to HTML. The trait surface (`StreamBuilder`,
/// `RenderHtml::to_html_*`) is referenced unconditionally; web-only
/// internals are gated on `feature = "web"` within the module.
pub mod ssr;
/// Types for SVG (web only). Native targets get their `<view>` etc.
/// element constructors from the per-renderer glue crate's
/// `view_prelude::__leptos_view::elements` namespace; `tachys::svg`
/// is unused on native after the Phase 4 macro refactor.
#[cfg(feature = "web")]
pub mod svg;
/// Core logic for manipulating views.
pub mod view;

pub use either_of as either;
#[cfg(all(feature = "islands", feature = "web"))]
#[doc(hidden)]
pub use wasm_bindgen;
#[cfg(all(feature = "islands", feature = "web"))]
#[doc(hidden)]
pub use web_sys;

/// View implementations for the `oco_ref` crate (cheaply-cloned string types).
#[cfg(all(feature = "oco", feature = "web"))]
pub mod oco;
/// View implementations for the `reactive_graph` crate.
#[cfg(feature = "reactive_graph")]
pub mod reactive_graph;

/// A type-erased container.
pub mod erased;

#[cfg(feature = "web")]
pub(crate) trait UnwrapOrDebug {
    type Output;

    fn or_debug(self, el: &Node, label: &'static str);

    fn ok_or_debug(
        self,
        el: &Node,
        label: &'static str,
    ) -> Option<Self::Output>;
}

#[cfg(feature = "web")]
impl<T> UnwrapOrDebug for Result<T, JsValue> {
    type Output = T;

    #[track_caller]
    fn or_debug(self, el: &Node, name: &'static str) {
        #[cfg(any(debug_assertions, leptos_debuginfo))]
        {
            if let Err(err) = self {
                let location = std::panic::Location::caller();
                web_sys::console::warn_3(
                    &JsValue::from_str(&format!(
                        "[WARNING] Non-fatal error at {location}, while \
                         calling {name} on "
                    )),
                    el,
                    &err,
                );
            }
        }
        #[cfg(not(any(debug_assertions, leptos_debuginfo)))]
        {
            _ = self;
        }
    }

    #[track_caller]
    fn ok_or_debug(
        self,
        el: &Node,
        name: &'static str,
    ) -> Option<Self::Output> {
        #[cfg(any(debug_assertions, leptos_debuginfo))]
        {
            if let Err(err) = &self {
                let location = std::panic::Location::caller();
                web_sys::console::warn_3(
                    &JsValue::from_str(&format!(
                        "[WARNING] Non-fatal error at {location}, while \
                         calling {name} on "
                    )),
                    el,
                    err,
                );
            }
            self.ok()
        }
        #[cfg(not(any(debug_assertions, leptos_debuginfo)))]
        {
            self.ok()
        }
    }
}

#[doc(hidden)]
#[macro_export]
macro_rules! or_debug {
    ($action:expr, $el:expr, $label:literal) => {
        if cfg!(any(debug_assertions, leptos_debuginfo)) {
            $crate::UnwrapOrDebug::or_debug($action, $el, $label);
        } else {
            _ = $action;
        }
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! ok_or_debug {
    ($action:expr, $el:expr, $label:literal) => {
        if cfg!(any(debug_assertions, leptos_debuginfo)) {
            $crate::UnwrapOrDebug::ok_or_debug($action, $el, $label)
        } else {
            $action.ok()
        }
    };
}
