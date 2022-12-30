//! # Leptos Router
//!
//! Leptos Router is a router and state management tool for web applications
//! written in Rust using the [Leptos](https://github.com/gbj/leptos) web framework.
//! It is ”isomorphic,” i.e., it can be used for client-side applications/single-page
//! apps (SPAs), server-side rendering/multi-page apps (MPAs), or to synchronize
//! state between the two.
//!
//! **Note:** This is a work in progress. The feature to pass client-side route [State] in
//! [History.state](https://developer.mozilla.org/en-US/docs/Web/API/History/state), in particular,
//! is incomplete.
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
//! #[component]
//! pub fn RouterExample(cx: Scope) -> impl IntoView {
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
//!               view=move |cx| view! { cx,  <ContactList/> }
//!             >
//!               // users like /gbj or /bob
//!               <Route
//!                 path=":id"
//!                 view=move |cx| view! { cx,  <Contact/> }
//!               />
//!               // a fallback if the /:id segment is missing from the URL
//!               <Route
//!                 path=""
//!                 view=move |_| view! { cx,  <p class="contact">"Select a contact."</p> }
//!               />
//!             </Route>
//!             // LR will automatically use this for /about, not the /:id match above
//!             <Route
//!               path="about"
//!               view=move |cx| view! { cx,  <About/> }
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
//! fn ContactList(cx: Scope) -> impl IntoView {
//!   // loads the contact list data once; doesn't reload when nested routes change
//!   let contacts = create_resource(cx, || (), |_| contact_list_data());
//!   view! {
//!     cx,
//!     <div>
//!       // show the contacts
//!       <ul>
//!         {move || contacts.read().map(|contacts| view! { cx, <li>"todo contact info"</li> } )}
//!       </ul>
//!
//!       // insert the nested child route here
//!       <Outlet/>
//!     </div>
//!   }
//! }
//!
//! #[component]
//! fn Contact(cx: Scope) -> impl IntoView {
//!   let params = use_params_map(cx);
//!   let data = create_resource(
//!     cx,
//!     move || params.with(|p| p.get("id").cloned().unwrap_or_default()),
//!     move |id| contact_data(id)
//!   );
//!   todo!()
//! }
//!
//! #[component]
//! fn About(cx: Scope) -> impl IntoView {
//!   todo!()
//! }
//!
//! ```

#![cfg_attr(not(feature = "stable"), feature(auto_traits))]
#![cfg_attr(not(feature = "stable"), feature(negative_impls))]
#![cfg_attr(not(feature = "stable"), feature(type_name_of_val))]

mod components;
mod history;
mod hooks;
mod matching;

pub use components::*;
pub use history::*;
pub use hooks::*;
pub use matching::*;
