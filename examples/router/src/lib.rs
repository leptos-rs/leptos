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
    params: ParamsMap,
    location: Location,
) -> Resource<String, Vec<ContactSummary>> {
    log::debug!("(contact_list) reloading contact list");
    create_resource(cx, location.search, move |s| get_contacts(s.to_string()))
}

fn contact(
    cx: Scope,
    params: ParamsMap,
    location: Location,
) -> Resource<Option<usize>, Option<Contact>> {
    log::debug!("(contact) reloading contact");
    create_resource(
        cx,
        move || {
            params
                .get("id")
                .cloned()
                .unwrap_or_default()
                .parse::<usize>()
                .ok()
        },
        move |id| get_contact(id),
    )
}

pub fn router_example(cx: Scope) -> Element {
    view! {
        <div>
            <nav>
                <a href="/">"Contacts"</a>
                <a href="/about">"About"</a>
                <a href="/settings">"Settings"</a>
            </nav>
            <main>
                <Router
                    mode=BrowserIntegration {}
                    base="/"
                >
                    <Routes>
                        <Route
                            path=""
                            element=move || view! { <ContactList/> }
                            loader=contact_list.into()
                        >
                            <Route
                                path=":id"
                                loader=contact.into()
                                element=move || view! { <Contact/> }
                            />
                        </Route>
                        <Route
                            path="about"
                            element=move || view! { <About/> }
                        />
                        <Route
                            path="settings"
                            element=move || view! { <Settings/> }
                        />
                    </Routes>
                </Router>
            </main>
        </div>
    }
}

#[component]
pub fn ContactList(cx: Scope) -> Vec<Element> {
    let contacts = use_loader::<Resource<String, Vec<ContactSummary>>>(cx);

    view! {
        <>
            <h1>"Contacts"</h1>
            <ul>
                <For each={move || contacts.read().unwrap_or_default()} key=|contact| contact.id>
                {|cx, contact: &ContactSummary| {
                    view! {
                        <li><a href=format!("/contacts/{}", contact.id)> {&contact.first_name} " " {&contact.last_name}</a></li>
                    }
                }}
                </For>
            </ul>
            <div><Outlet/></div>
        </>
    }
}

#[component]
pub fn Contact(cx: Scope) -> Element {
    //let contact = use_loader::<Resource<Option<usize>, Option<Contact>>>(cx);

    view! {
        <pre>"Contact info here"</pre>
    }
}

#[component]
pub fn About(cx: Scope) -> Vec<Element> {
    view! {
        <>
            <h1>"About"</h1>
            <p>"Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat. Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur. Excepteur sint occaecat cupidatat non proident, sunt in culpa qui officia deserunt mollit anim id est laborum."</p>
        </>
    }
}

#[component]
pub fn Settings(cx: Scope) -> Vec<Element> {
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
