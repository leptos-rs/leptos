mod api;

use api::{Contact, ContactSummary};
use leptos::*;
use leptos_router::*;

use crate::api::{get_contact, get_contacts};

async fn contact_list_data(_cx: Scope, _params: ParamsMap, url: Url) -> Vec<ContactSummary> {
    log::debug!("(contact_list_data) reloading contacts");
    get_contacts(url.search).await
}

async fn contact_data(_cx: Scope, params: ParamsMap, _url: Url) -> Option<Contact> {
    log::debug!("(contact_data) reloading contact");
    let id = params
        .get("id")
        .cloned()
        .unwrap_or_default()
        .parse::<usize>()
        .ok();
    get_contact(id).await
}

pub fn router_example(cx: Scope) -> Element {
    view! {
        <div id="root">
            <Router>
                <nav>
                    <A href="contacts">"Contacts"</A>
                    <A href="about">"About"</A>
                    <A href="settings">"Settings"</A>
                </nav>
                <main>
                    <Routes>
                        <Route
                            path=""
                            element=move |cx| view! { <ContactList/> }
                            loader=contact_list_data.into()
                        >
                            <Route
                                path=":id"
                                loader=contact_data.into()
                                element=move |cx| view! { <Contact/> }
                            />
                            <Route
                                path="about"
                                element=move |_| view! { <p class="contact">"Here is your list of contacts"</p> }
                            />
                            <Route
                                path=""
                                element=move |_| view! { <p class="contact">"Select a contact."</p> }
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
                    </Routes>
                </main>
            </Router>
        </div>
    }
}

#[component]
pub fn ContactList(cx: Scope) -> Element {
    let contacts = use_loader::<Vec<ContactSummary>>(cx);
    log::debug!("rendering <ContactList/>");

    view! {
        <div class="contact-list">
            <h1>"Contacts"</h1>
            <ul>
                <Suspense fallback=move || view! { <p>"Loading contacts..."</p> }>{
                    move || {
                        contacts.read().map(|contacts| view! {
                            <For each=move || contacts.clone() key=|contact| contact.id>
                                {move |cx, contact: &ContactSummary| {
                                    let id = contact.id;
                                    let name = format!("{} {}", contact.first_name, contact.last_name);
                                    view! {
                                        <li><A href=id.to_string()><span>{name.clone()}</span></A></li>
                                    }
                                }}
                            </For>
                        })
                    }
                }</Suspense>
            </ul>
            <Outlet/>
        </div>
    }
}

#[component]
pub fn Contact(cx: Scope) -> Element {
    let contact = use_loader::<Option<Contact>>(cx);

    view! {
        <div class="contact">
            <Suspense fallback=move || view! { <p>"Loading..."</p> }>{
                move || contact.read().map(|contact| contact.map(|contact| view! {
                    <section class="card">
                        <h1>{contact.first_name} " " {contact.last_name}</h1>
                        <p>{contact.address_1}<br/>{contact.address_2}</p>
                    </section>
                }))
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
