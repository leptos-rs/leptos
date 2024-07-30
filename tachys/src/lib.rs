#![allow(incomplete_features)] // yolo
#![cfg_attr(feature = "nightly", feature(adt_const_params))]

pub mod prelude {
    #[cfg(feature = "reactive_graph")]
    pub use crate::reactive_graph::FutureViewExt;
    pub use crate::{
        html::{
            attribute::{
                aria::AriaAttributes,
                custom::CustomAttribute,
                global::{
                    ClassAttribute, GlobalAttributes, OnAttribute,
                    OnTargetAttribute, PropAttribute, StyleAttribute,
                },
            },
            directive::DirectiveAttribute,
            element::{ElementChild, InnerHtmlAttribute},
            node_ref::NodeRefAttribute,
        },
        renderer::{dom::Dom, Renderer},
        view::{
            add_attr::AddAnyAttr, any_view::IntoAny, Mountable, Render,
            RenderHtml,
        },
    };
}

use wasm_bindgen::JsValue;
use web_sys::Node;

pub mod dom;
pub mod html;
pub mod hydration;
pub mod mathml;
pub mod renderer;
pub mod ssr;
pub mod svg;
pub mod view;

pub use either_of as either;
#[cfg(feature = "islands")]
pub use wasm_bindgen;
#[cfg(feature = "islands")]
pub use web_sys;

#[cfg(feature = "oco")]
pub mod oco;
#[cfg(feature = "reactive_graph")]
pub mod reactive_graph;

pub fn log(text: &str) {
    web_sys::console::log_1(&JsValue::from_str(text));
}

pub(crate) trait UnwrapOrDebug {
    type Output;

    fn or_debug(self, el: &Node, label: &'static str);

    fn ok_or_debug(
        self,
        el: &Node,
        label: &'static str,
    ) -> Option<Self::Output>;
}

impl<T> UnwrapOrDebug for Result<T, JsValue> {
    type Output = T;

    #[track_caller]
    fn or_debug(self, el: &Node, name: &'static str) {
        #[cfg(debug_assertions)]
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
        #[cfg(not(debug_assertions))]
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
        #[cfg(debug_assertions)]
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
        #[cfg(not(debug_assertions))]
        {
            self.ok()
        }
    }
}

#[macro_export]
macro_rules! or_debug {
    ($action:expr, $el:expr, $label:literal) => {
        if cfg!(debug_assertions) {
            $crate::UnwrapOrDebug::or_debug($action, $el, $label);
        } else {
            _ = $action;
        }
    };
}

#[macro_export]
macro_rules! ok_or_debug {
    ($action:expr, $el:expr, $label:literal) => {
        if cfg!(debug_assertions) {
            $crate::UnwrapOrDebug::ok_or_debug($action, $el, $label)
        } else {
            $action.ok()
        }
    };
}
