mod api;

use std::{
    any::{Any, TypeId},
    time::Duration,
};

use api::{Contact, ContactSummary};
use futures::Future;
use leptos::*;

use crate::api::{get_contact, get_contacts};

fn contact_list(
    cx: Scope,
    params: Memo<ParamsMap>,
    location: Location,
) -> Resource<String, Vec<ContactSummary>> {
    log::debug!("(contact_list) reloading contact list");
    create_resource(cx, location.search, get_contacts)
}

fn contact(
    cx: Scope,
    params: Memo<ParamsMap>,
    location: Location,
) -> Resource<Option<usize>, Option<Contact>> {
    log::debug!("(contact) reloading contact");
    create_resource(
        cx,
        move || {
            params()
                .get("id")
                .cloned()
                .unwrap_or_default()
                .parse::<usize>()
                .ok()
        },
        get_contact,
    )
}

pub fn router_example(cx: Scope) -> Element {
    view! {
        <div>
            <Router
                mode=BrowserIntegration {}
            >
                <Routes>
                    <Route path=""
                        element=move |cx| view! { <Index/> }
                    >
                        <Route
                            path="contacts"
                            element=move |cx| view! { <ContactList/> }
                            loader=contact_list.into()
                        >
                            <Route
                                path=":id"
                                loader=contact.into()
                                element=move |cx| view! { <Contact/> }
                            />
                            <Route
                                path="about"
                                loader=contact.into()
                                element=move |cx| view! { <p class="contact">"Here is your list of contacts"</p> }
                            />
                            <Route
                                path=""
                                loader=contact.into()
                                element=move |cx| view! { <p class="contact">"Select a contact."</p> }
                            />
                        </Route>
                        <Route
                            path="about"
                            element=move |cx| view! { <About/> }
                        />
                        <Route
                            path="settings"
                            element=move |cx| view! { <Settings/> }
                        />
                    </Route>
                </Routes>
            </Router>
        </div>
    }
}

#[component]
pub fn Index(cx: Scope) -> Vec<Element> {
    view! {
        <>
            <nav>
                <NavLink to="contacts".into()>"Contacts"</NavLink>
                <NavLink to="about".into()>"About"</NavLink>
                <NavLink to="settings".into()>"Settings"</NavLink>
            </nav>
            <main><Outlet/></main>
        </>
    }
}

#[component]
pub fn ContactList(cx: Scope) -> Element {
    let contacts = use_loader::<Resource<String, Vec<ContactSummary>>>(cx);

    log::debug!(
        "[ContactList] before <Suspense/>, use_route(cx).path() is {:?}",
        use_route(cx).path()
    );

    view! {
        <div class="contact-list">
            <h1>"Contacts"</h1>
            <Link to="about".into()>"About"</Link>
            <NavLink to={0.to_string()}>"Link to first contact"</NavLink>
            <ul>
                <Suspense fallback=move || view! { <p>"Loading contacts..."</p> }>{
                    move || {
                        log::debug!("[ContactList] inside <Suspense/>, use_route(cx) is now {:?}", use_route(cx).path());
                        view! {
                            <For each={move || contacts.read().unwrap_or_default()} key=|contact| contact.id>
                                {move |cx, contact: &ContactSummary| {
                                    view! {
                                        <li><NavLink to={contact.id.to_string()}><span>{&contact.first_name} " " {&contact.last_name}</span></NavLink></li>
                                    }
                                }}
                                </For>
                        }
                    }
                }</Suspense>
            </ul>
            <Outlet/>
        </div>
    }
}

#[component]
pub fn Contact(cx: Scope) -> Element {
    let contact = use_loader::<Resource<Option<usize>, Option<Contact>>>(cx);

    view! {
        <div class="contact">
            <Suspense fallback=move || view! { <p>"Loading..."</p> }>{
                move || contact.read().flatten().map(|contact| view! {
                    <section class="card">
                        <h1>{contact.first_name} " " {contact.last_name}</h1>
                        <p>{contact.address_1}<br/>{contact.address_2}</p>
                    </section>
                })
            }</Suspense>
        </div>
    }
}

#[component]
pub fn About(_cx: Scope) -> Vec<Element> {
    view! {
        <>
            <h1>"About"</h1>
            <p>"Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat. Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur. Excepteur sint occaecat cupidatat non proident, sunt in culpa qui officia deserunt mollit anim id est laborum."</p>
        </>
    }
}

#[component]
pub fn Settings(_cx: Scope) -> Vec<Element> {
    view! {
        <>
            <h1>"Settings"</h1>
            <form>
                <fieldset>
                    <legend>"Name"</legend>
                    <input type="text" name="first_name" placeholder="First"/>
                    <input type="text" name="first_name" placeholder="Last"/>
                </fieldset>
                <pre>"This page is just a placeholder."</pre>
            </form>
        </>
    }
}
