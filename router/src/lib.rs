//! # Leptos Router
//!
//! Leptos Router is a router and state management tool for web applications
//! written in Rust using the [Leptos](https://github.com/gbj/leptos) web framework.
//! It is ”isomorphic,” i.e., it can be used for client-side applications/single-page
//! apps (SPAs), server-side rendering/multi-page apps (MPAs), or to synchronize
//! state between the two.
//!
//! **Note:** This is a work in progress. Docs are still being written,
//! and some features are only stubs, in particular
//! - passing client-side route [State] in [History.state](https://developer.mozilla.org/en-US/docs/Web/API/History/state))
//! - data mutations using [Action]s and [Form] `method="POST"`
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
//! 3. **Route-based data loading.** Each route should know exactly which data it needs
//!    to render itself when the route is defined. This allows each route’s data to be
//!    reloaded independently, and allows data from nested routes to be loaded in parallel,
//!    avoiding waterfalls.
//!
//! 4. **Progressive enhancement.** The [A] and [Form] components resolve any relative
//!    nested routes, render actual `<a>` and `<form>` elements, and (when possible)
//!    upgrading them to handle those navigations with client-side routing. If you’re using
//!    them with server-side rendering (with or without hydration), they just work,
//!    whether JS/WASM have loaded or not.
//!
//! ## Example
//!
//! ```rust
//!
//! use leptos::*;
//! use leptos_router::*;
//!
//! pub fn router_example(cx: Scope) -> Element {
//!   view! {
//!     cx,
//!     <div id="root">
//!       // we wrap the whole app in a <Router/> to allow client-side navigation
//!       // from our nav links below
//!       <Router>
//!         // <nav> and <main> will show on every route
//!         <nav>
//!           // LR will enhance the active <a> link with the [aria-current] attribute
//!           // we can use this for styling them with CSS like `[aria-current] { font-weight: bold; }`
//!           <A href="contacts">"Contacts"</A>
//!           <A href="about">"About"</A>
//!           <A href="settings">"Settings"</A>
//!         </nav>
//!         <main>
//!           // <Routes/> both defines our routes and shows them on the page
//!           <Routes>
//!             // our root route: the contact list is always shown
//!             <Route
//!               path=""
//!               element=move |cx| view! { cx,  <ContactList/> }
//!               // <ContactList/> needs all the contacts, so we provide the loader here
//!               // this will only be reloaded if we navigate away to /about and back to / or /:id
//!               loader=contact_list_data.into()
//!             >
//!               // users like /gbj or /bob
//!               <Route
//!                 path=":id"
//!                 // <Contact/> needs contact data, so we provide the loader here
//!                 // this will be reloaded when the :id changes
//!                 loader=contact_data.into()
//!                 element=move |cx| view! { cx,  <Contact/> }
//!               />
//!               // a fallback if the /:id segment is missing from the URL
//!               // doesn't need any data, so no loader is provided
//!               <Route
//!                 path=""
//!                 element=move |_| view! { cx,  <p class="contact">"Select a contact."</p> }
//!               />
//!             </Route>
//!             // LR will automatically use this for /about, not the /:id match above
//!             <Route
//!               path="about"
//!               element=move |cx| view! { cx,  <About/> }
//!             />
//!           </Routes>
//!         </main>
//!       </Router>
//!     </div>
//!   }
//! }
//!
//! // Loaders are async functions that have access to the reactive scope,
//! // map of matched URL params for that route, and the URL
//! // They are reloaded whenever the params or URL change
//!
//! type ContactSummary = (); // TODO!
//! type Contact = (); // TODO!()
//!
//! // contact_data reruns whenever the :id param changes
//! async fn contact_data(_cx: Scope, _params: ParamsMap, url: Url) -> Contact {
//!   todo!()
//! }
//!
//! // contact_list_data *doesn't* rerun when the :id changes,
//! // because that param is nested lower than the <ContactList/> route
//! async fn contact_list_data(_cx: Scope, _params: ParamsMap, url: Url) -> Vec<ContactSummary> {
//!   todo!()
//! }
//!
//! #[component]
//! fn ContactList(cx: Scope) -> Element {
//!   let data = use_loader::<Vec<ContactSummary>>(cx);
//!   todo!()
//! }
//!
//! #[component]
//! fn Contact(cx: Scope) -> Element {
//!   let data = use_loader::<Contact>(cx);
//!   todo!()
//! }
//!
//! #[component]
//! fn About(cx: Scope) -> Element {
//!   todo!()
//! }
//!
//! ```

#![feature(auto_traits)]
#![feature(let_chains)]
#![feature(negative_impls)]
#![feature(type_name_of_val)]

mod components;
mod data;
mod error;
mod fetch;
mod history;
mod hooks;
mod matching;

pub use components::*;
pub use data::*;
pub use error::*;
pub use fetch::*;
pub use history::*;
pub use hooks::*;
pub use matching::Branch;
