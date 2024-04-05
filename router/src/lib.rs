#![forbid(unsafe_code)]

//! # Leptos Router
//!
//! Leptos Router is a router and state management tool for web applications
//! written in Rust using the [`Leptos`] web framework.
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
//! 3. **Progressive enhancement.** The [`A`] and [`Form`] components resolve any relative
//!    nested routes, render actual `<a>` and `<form>` elements, and (when possible)
//!    upgrading them to handle those navigations with client-side routing. If you’re using
//!    them with server-side rendering (with or without hydration), they just work,
//!    whether JS/WASM have loaded or not.
//!
//! ## Example
//!
//! ```rust
//! use leptos::*;
//! use leptos_router::*;
//!
//! #[component]
//! pub fn RouterExample() -> impl IntoView {
//!   view! {
//!
//!     <div id="root">
//!       // we wrap the whole app in a <Router/> to allow client-side navigation
//!       // from our nav links below
//!       <Router>
//!         // <nav> and <main> will show on every route
//!         <nav>
//!           // LR will enhance the active <a> link with the [aria-current] attribute
//!           // we can use this for styling them with CSS like `[aria-current] { font-weight: bold; }`
//!           <A href="contacts">"Contacts"</A>
//!           // But we can also use a normal class attribute like it is a normal component
//!           <A href="settings" class="my-class">"Settings"</A>
//!           // It also supports signals!
//!           <A href="about" class=move || "my-class">"About"</A>
//!         </nav>
//!         <main>
//!           // <Routes/> both defines our routes and shows them on the page
//!           <Routes>
//!             // our root route: the contact list is always shown
//!             <Route
//!               path=""
//!               view=ContactList
//!             >
//!               // users like /gbj or /bob
//!               <Route
//!                 path=":id"
//!                 view=Contact
//!               />
//!               // a fallback if the /:id segment is missing from the URL
//!               <Route
//!                 path=""
//!                 view=move || view! { <p class="contact">"Select a contact."</p> }
//!               />
//!             </Route>
//!             // LR will automatically use this for /about, not the /:id match above
//!             <Route
//!               path="about"
//!               view=About
//!             />
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
//!   let contacts = create_resource(|| (), |_| contact_list_data());
//!   view! {
//!
//!     <div>
//!       // show the contacts
//!       <ul>
//!         {move || contacts.read().map(|contacts| view! { <li>"todo contact info"</li> } )}
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
//!   let data = create_resource(
//!
//!     move || params.with(|p| p.get("id").cloned().unwrap_or_default()),
//!     move |id| contact_data(id)
//!   );
//!   todo!()
//! }
//!
//! #[component]
//! fn About() -> impl IntoView {
//!   todo!()
//! }
//! ```
//!
//! ## Module Route Definitions
//! Routes can also be modularized and nested by defining them in separate components, which can be
//! located in and imported from other modules. Components that return `<Route/>` should be marked
//! `#[component(transparent)]`, as in this example:
//! ```rust
//! use leptos::*;
//! use leptos_router::*;
//!
//! #[component]
//! pub fn App() -> impl IntoView {
//!   view! {
//!     <Router>
//!       <Routes>
//!         <Route path="/" view=move || {
//!           view! { "-> /" }
//!         }/>
//!         <ExternallyDefinedRoute/>
//!       </Routes>
//!     </Router>
//!   }
//! }
//!
//! // `transparent` here marks the component as returning data (a RouteDefinition), not a view
//! #[component(transparent)]
//! pub fn ExternallyDefinedRoute() -> impl IntoView {
//!   view! {
//!     <Route path="/some-area" view=move || {
//!       view! { <div>
//!         <h2>"Some Area"</h2>
//!         <Outlet/>
//!       </div> }
//!     }>
//!       <Route path="/path-a/:id" view=move || {
//!         view! { <p>"Path A"</p> }
//!       }/>
//!       <Route path="/path-b/:id" view=move || {
//!         view! { <p>"Path B"</p> }
//!       }/>
//!     </Route>
//!   }
//! }
//! ```
//!
//! # Feature Flags
//! - `csr` Client-side rendering: Generate DOM nodes in the browser
//! - `ssr` Server-side rendering: Generate an HTML string (typically on the server)
//! - `hydrate` Hydration: use this to add interactivity to an SSRed Leptos app
//! - `nightly`: On `nightly` Rust, enables the function-call syntax for signal getters and setters.
//!
//! **Important Note:** You must enable one of `csr`, `hydrate`, or `ssr` to tell Leptos
//! which mode your app is operating in.
//!
//! [`Leptos`]: <https://github.com/leptos-rs/leptos>

#![cfg_attr(feature = "nightly", feature(auto_traits))]
#![cfg_attr(feature = "nightly", feature(negative_impls))]
#![cfg_attr(feature = "nightly", feature(type_name_of_val))]
// to prevent warnings from popping up when a nightly feature is stabilized
#![allow(stable_features)]

mod animation;
mod components;
#[cfg(any(feature = "ssr", doc))]
mod extract_routes;
mod history;
mod hooks;
#[doc(hidden)]
pub mod matching;
mod render_mode;
pub use components::*;
#[cfg(any(feature = "ssr", doc))]
pub use extract_routes::*;
pub use history::*;
pub use hooks::*;
pub use matching::{RouteDefinition, *};
pub use render_mode::*;
extern crate tracing;
