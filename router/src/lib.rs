#![forbid(unsafe_code)]
#![cfg_attr(feature = "nightly", feature(auto_traits))]
#![cfg_attr(feature = "nightly", feature(negative_impls))]

pub mod components;
pub mod flat_router;
mod form;
mod generate_route_list;
pub mod hooks;
mod link;
pub mod location;
mod matching;
mod method;
mod navigate;
pub mod nested_router;
pub mod params;
mod ssr_mode;
pub mod static_routes;

pub use generate_route_list::*;
#[doc(inline)]
pub use leptos_router_macro::path;
pub use matching::*;
pub use method::*;
pub use navigate::*;
pub use ssr_mode::*;

pub(crate) mod view_transition {
    use js_sys::{Function, Promise, Reflect};
    use leptos::leptos_dom::helpers::document;
    use wasm_bindgen::{closure::Closure, intern, JsCast, JsValue};

    pub fn start_view_transition(
        level: u8,
        is_back_navigation: bool,
        fun: impl FnOnce() + 'static,
    ) {
        let document = document();
        let document_element = document.document_element().unwrap();
        let class_list = document_element.class_list();
        let svt = Reflect::get(
            &document,
            &JsValue::from_str(intern("startViewTransition")),
        )
        .and_then(|svt| svt.dyn_into::<Function>());
        _ = class_list.add_1(&format!("router-outlet-{level}"));
        if is_back_navigation {
            _ = class_list.add_1("router-back");
        }
        match svt {
            Ok(svt) => {
                let cb = Closure::once_into_js(Box::new(move || {
                    fun();
                }));
                match svt.call1(
                    document.unchecked_ref(),
                    cb.as_ref().unchecked_ref(),
                ) {
                    Ok(view_transition) => {
                        let class_list = document_element.class_list();
                        let finished = Reflect::get(
                            &view_transition,
                            &JsValue::from_str("finished"),
                        )
                        .expect("no `finished` property on ViewTransition")
                        .unchecked_into::<Promise>();
                        let cb = Closure::new(Box::new(move |_| {
                            if is_back_navigation {
                                class_list.remove_1("router-back").unwrap();
                            }
                            class_list
                                .remove_1(&format!("router-outlet-{level}"))
                                .unwrap();
                        })
                            as Box<dyn FnMut(JsValue)>);
                        _ = finished.then(&cb);
                        cb.into_js_value();
                    }
                    Err(e) => {
                        web_sys::console::log_1(&e);
                    }
                }
            }
            Err(_) => {
                leptos::logging::warn!(
                    "NOTE: View transitions are not supported in this \
                     browser; unless you provide a polyfill, view transitions \
                     will not be applied."
                );
                fun();
            }
        }
    }
}
