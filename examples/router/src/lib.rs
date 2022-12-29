mod api;

use leptos::*;
use leptos_router::*;

use crate::api::{get_contact, get_contacts};

#[component]
pub fn RouterExample(cx: Scope) -> impl IntoView {
    log::debug!("rendering <RouterExample/>");

    view! { cx,
        <Router>
            <nav>
                <A exact=true href="/">"Contacts"</A>
                <A href="about">"About"</A>
                <A href="settings">"Settings"</A>
            </nav>
            <main>
                <Routes>
                    <Route
                        path=""
                        view=move |cx| view! { cx,  <ContactList/> }
                    >
                        <Route
                            path=":id"
                            view=move |cx| view! { cx,  <Contact/> }
                        />
                        <Route
                            path="/"
                            view=move |_| view! { cx,  <p>"Select a contact."</p> }
                        />
                    </Route>
                    <Route
                        path="about"
                        view=move |cx| view! { cx,  <About/> }
                    />
                    <Route
                        path="settings"
                        view=move |cx| view! { cx,  <Settings/> }
                    />
                </Routes>
            </main>
        </Router>
    }
}

#[component]
pub fn ContactList(cx: Scope) -> impl IntoView {
    log::debug!("rendering <ContactList/>");

    let location = use_location(cx);
    let contacts = create_resource(cx, move || location.search.get(), get_contacts);
    let contacts = move || {
        contacts.read().map(|contacts| {
            // this data doesn't change frequently so we can use .map().collect() instead of a keyed <For/>
            contacts
                .into_iter()
                .map(|contact| {
                    view! { cx,
                        <li><A href=contact.id.to_string()><span>{&contact.first_name} " " {&contact.last_name}</span></A></li>
                    }
                })
                .collect::<Vec<_>>()
        })
    };

    view! { cx,
        <div class="contact-list">
            <h1>"Contacts"</h1>
            <Suspense fallback=move || view! { cx,  <p>"Loading contacts..."</p> }>
                {move || view! { cx, <ul>{contacts}</ul>}}
            </Suspense>
            <Outlet/>
        </div>
    }
}

#[component]
pub fn Contact(cx: Scope) -> impl IntoView {
    log::debug!("rendering <Contact/>");

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
        Some(None) => Some(view! { cx, <p>"No contact with this ID was found."</p> }.into_any()),
        // Some(Some) => has loaded and found a contact
        Some(Some(contact)) => Some(view! { cx,
            <section class="card">
                <h1>{contact.first_name} " " {contact.last_name}</h1>
                <p>{contact.address_1}<br/>{contact.address_2}</p>
            </section>
        }.into_any()),
    };

    view! { cx,
        <div class="contact">
            <Transition fallback=move || view! { cx,  <p>"Loading..."</p> }>
                {contact_display}
            </Transition>
        </div>
    }
}

#[component]
pub fn About(cx: Scope) -> impl IntoView {
    log::debug!("rendering <About/>");

    view! { cx,
        <>
            <h1>"About"</h1>
            <p>"Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat. Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur. Excepteur sint occaecat cupidatat non proident, sunt in culpa qui officia deserunt mollit anim id est laborum."</p>
        </>
    }
}

#[component]
pub fn Settings(cx: Scope) -> impl IntoView {
    log::debug!("rendering <Settings/>");
    view! { cx,
        <>
            <h1>"Settings"</h1>
            <form>
                <fieldset>
                    <legend>"Name"</legend>
                    <input type="text" name="first_name" placeholder="First"/>
                    <input type="text" name="last_name" placeholder="Last"/>
                </fieldset>
                <pre>"This page is just a placeholder."</pre>
            </form>
        </>
    }
}
