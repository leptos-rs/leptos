//! # Leptos Router
//!
//! Leptos Router is a router and state management tool for web applications
//! written in Rust using the Leptos web framework.
//! It is ”isomorphic”, i.e., it can be used for client-side applications/single-page
//! apps (SPAs), server-side rendering/multi-page apps (MPAs), or to synchronize
//! state between the two.
//!
//! ## Philosophy
//!
//! Leptos Router is built on a few simple principles:
//! 1. **URL drives state.** For web applications, the URL should be the ultimate
//!    source of truth for most of your app’s state. (It’s called a **Universal
//!    Resource Locator** for a reason!)
//!
//! 2. **Nested routing.** A URL can match multiple routes that exist in a nested tree
//!    and are rendered by different components. This means you can navigate between siblings
//!    in this tree without re-rendering or triggering any change in the parent routes.
//!
//! 3. **Progressive enhancement.** The [`A`](crate::components::A) and
//!    [`Form`](crate::components::Form) components resolve any relative
//!    nested routes, render actual `<a>` and `<form>` elements, and (when possible)
//!    upgrading them to handle those navigations with client-side routing. If you’re using
//!    them with server-side rendering (with or without hydration), they just work,
//!    whether JS/WASM have loaded or not.
//!
//! ## Example
//!
//! ```rust
//! use leptos::prelude::*;
//! use leptos_router::components::*;
//! use leptos_router::path;
//! use leptos_router::hooks::use_params_map;
//!
//! #[component]
//! pub fn RouterExample() -> impl IntoView {
//!   view! {
//!
//!     <div id="root">
//!       // we wrap the whole app in a <Router/> to allow client-side navigation
//!       // from our nav links below
//!       <Router>
//!         <main>
//!           // <Routes/> both defines our routes and shows them on the page
//!           <Routes fallback=|| "Not found.">
//!             // our root route: the contact list is always shown
//!             <ParentRoute
//!               path=path!("")
//!               view=ContactList
//!             >
//!               // users like /gbj or /bob
//!               <Route
//!                 path=path!(":id")
//!                 view=Contact
//!               />
//!               // a fallback if the /:id segment is missing from the URL
//!               <Route
//!                 path=path!("")
//!                 view=move || view! { <p class="contact">"Select a contact."</p> }
//!               />
//!             </ParentRoute>
//!           </Routes>
//!         </main>
//!       </Router>
//!     </div>
//!   }
//! }
//!
//! type ContactSummary = (); // TODO!
//! type Contact = (); // TODO!()
//!
//! // contact_data reruns whenever the :id param changes
//! async fn contact_data(id: String) -> Contact {
//!   todo!()
//! }
//!
//! // contact_list_data *doesn't* rerun when the :id changes,
//! // because that param is nested lower than the <ContactList/> route
//! async fn contact_list_data() -> Vec<ContactSummary> {
//!   todo!()
//! }
//!
//! #[component]
//! fn ContactList() -> impl IntoView {
//!   // loads the contact list data once; doesn't reload when nested routes change
//!   let contacts = Resource::new(|| (), |_| contact_list_data());
//!   view! {
//!
//!     <div>
//!       // show the contacts
//!       <ul>
//!         {move || contacts.get().map(|contacts| view! { <li>"todo contact info"</li> } )}
//!       </ul>
//!
//!       // insert the nested child route here
//!       <Outlet/>
//!     </div>
//!   }
//! }
//!
//! #[component]
//! fn Contact() -> impl IntoView {
//!   let params = use_params_map();
//!   let data = Resource::new(
//!     move || params.read().get("id").unwrap_or_default(),
//!     move |id| contact_data(id)
//!   );
//!   todo!()
//! }
//! ```
//!
//! You can find examples of additional APIs in the [`router`] example.
//!
//! # Feature Flags
//! - `ssr` Server-side rendering: Generate an HTML string (typically on the server)
//! - `nightly`: On `nightly` Rust, enables the function-call syntax for signal getters and setters.
//! - `tracing`: Enables support for the `tracing` crate.
//!
//! [`Leptos`]: <https://github.com/leptos-rs/leptos>
//! [`router`]: <https://github.com/leptos-rs/leptos/blob/main/examples/router/src/lib.rs>

#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![cfg_attr(all(feature = "nightly", rustc_nightly), feature(auto_traits))]
#![cfg_attr(all(feature = "nightly", rustc_nightly), feature(negative_impls))]

/// Components for route definition and for enhanced links and forms.
pub mod components;
/// An optimized "flat" router without nested routes.
pub mod flat_router;
mod form;
mod generate_route_list;
/// Hooks that can be used to access router state inside your components.
pub mod hooks;
mod link;
/// Utilities for accessing the current location.
pub mod location;
mod matching;
mod method;
mod navigate;
/// A nested router that supports multiple levels of route definitions.
pub mod nested_router;
/// Support for maps of parameters in the path or in the query.
pub mod params;
mod ssr_mode;
/// Support for static routing.
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
