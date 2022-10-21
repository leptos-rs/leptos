mod api;

use api::{Contact, ContactSummary};
use leptos::*;
use leptos_router::*;

use crate::api::{get_contact, get_contacts};

pub fn router_example(cx: Scope) -> Element {
    view! { cx,
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
                            element=move |cx| view! { cx,  <ContactList/> }
                        >
                            <Route
                                path=":id?"
                                element=move |cx| view! { cx,  <Contact/> }
                            />
                            <Route
                                path="about"
                                element=move |_| view! { cx,  <p class="contact">"Here is your list of contacts"</p> }
                            />
                        </Route>
                        <Route
                            path="about"
                            element=move |cx| view! { cx,  <About/> }
                        />
                        <Route
                            path="settings"
                            element=move |cx| view! { cx,  <Settings/> }
                        />
                    </Routes>
                </main>
            </Router>
        </div>
    }
}

#[component]
pub fn ContactList(cx: Scope) -> Element {
    let location = use_location(cx);
    let contacts = create_resource(cx, move || location.search.get(), get_contacts);

    view! { cx,
        <div class="contact-list">
            <h1>"Contacts"</h1>
            <ul>
                <Suspense fallback=move || view! { cx,  <p>"Loading contacts..."</p> }>{
                    move || {
                        contacts.read().map(|contacts| view! { cx,
                            <For each=move || contacts.clone() key=|contact| contact.id>
                                {move |cx, contact: &ContactSummary| {
                                    let id = contact.id;
                                    let name = format!("{} {}", contact.first_name, contact.last_name);
                                    view! { cx,
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
    let params = use_params_map(cx);
    let contact = create_resource(
        cx,
        move || {
            params()
                .get("id")
                .cloned()
                .unwrap_or_default()
                .parse::<usize>()
                .ok()
        },
        // any of the following would work (they're identical)
        // move |id| async move { get_contact(id).await }
        // move |id| get_contact(id),
        // get_contact
        get_contact,
    );

    let contact_display = move || match contact.read() {
        // None => loading, but will be caught by Suspense fallback
        // I'm only doing this explicitly for the example
        None => None,
        // Some(None) => has loaded and found no contact
        Some(None) => Some(view! { cx, <p>"Please select a contact."</p> }),
        // Some(Some) => has loaded and found a contact
        Some(Some(contact)) => Some(view! { cx,
            <section class="card">
                <h1>{contact.first_name} " " {contact.last_name}</h1>
                <p>{contact.address_1}<br/>{contact.address_2}</p>
            </section>
        }),
    };

    view! { cx,
        <div class="contact">
            <Suspense fallback=move || view! { cx,  <p>"Loading..."</p> }>
                {contact_display}
            </Suspense>
        </div>
    }
}

#[component]
pub fn About(_cx: Scope) -> Vec<Element> {
    view! { cx,
        <>
            <h1>"About"</h1>
            <p>"Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat. Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur. Excepteur sint occaecat cupidatat non proident, sunt in culpa qui officia deserunt mollit anim id est laborum."</p>
        </>
    }
}

#[component]
pub fn Settings(_cx: Scope) -> Vec<Element> {
    view! { cx,
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
