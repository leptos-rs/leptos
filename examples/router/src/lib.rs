mod api;
use crate::api::*;
use leptos::{logging::log, *};
use leptos_router::*;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
struct ExampleContext(i32);

#[component]
pub fn RouterExample() -> impl IntoView {
    log::debug!("rendering <RouterExample/>");

    // contexts are passed down through the route tree
    provide_context(ExampleContext(0));

    view! {
        <Router>
            <nav>
                // ordinary <a> elements can be used for client-side navigation
                // using <A> has two effects:
                // 1) ensuring that relative routing works properly for nested routes
                // 2) setting the `aria-current` attribute on the current link,
                //    for a11y and styling purposes
                <A exact=true href="/">"Contacts"</A>
                <A href="about">"About"</A>
                <A href="settings">"Settings"</A>
                <A href="redirect-home">"Redirect to Home"</A>
            </nav>
            <main>
                <AnimatedRoutes
                    outro="slideOut"
                    intro="slideIn"
                    outro_back="slideOutBack"
                    intro_back="slideInBack"
                 >
                    <ContactRoutes/>
                    <Route
                        path="about"
                        view=|| view! { <About/> }
                    />
                    <Route
                        path="settings"
                        view=|| view! { <Settings/> }
                    />
                    <Route
                        path="redirect-home"
                        view=|| view! { <Redirect path="/"/> }
                    />
                </AnimatedRoutes>
            </main>
        </Router>
    }
}

// You can define other routes in their own component.
// Use a #[component(transparent)] that returns a <Route/>.
#[component(transparent)]
pub fn ContactRoutes() -> impl IntoView {
    view! {
        <Route
            path=""
            view=|| view! { <ContactList/> }
        >
            <Route
                path=":id"
                view=|| view! { <Contact/> }
            />
            <Route
                path="/"
                view=|| view! {  <p>"Select a contact."</p> }
            />
        </Route>
    }
}

#[component]
pub fn ContactList() -> impl IntoView {
    log::debug!("rendering <ContactList/>");

    // contexts are passed down through the route tree
    provide_context(ExampleContext(42));

    on_cleanup(|| {
        log!("cleaning up <ContactList/>");
    });

    let location = use_location();
    let contacts = create_resource(move || location.search.get(), get_contacts);
    let contacts = move || {
        contacts.get().map(|contacts| {
            // this data doesn't change frequently so we can use .map().collect() instead of a keyed <For/>
            contacts
                .into_iter()
                .map(|contact| {
                    view! {
                        <li><A href=contact.id.to_string()><span>{&contact.first_name} " " {&contact.last_name}</span></A></li>
                    }
                })
                .collect_view()
        })
    };

    view! {
        <div class="contact-list">
            <h1>"Contacts"</h1>
            <Suspense fallback=move || view! {  <p>"Loading contacts..."</p> }>
                {move || view! { <ul>{contacts}</ul>}}
            </Suspense>
            <AnimatedOutlet
                class="outlet"
                outro="fadeOut"
                intro="fadeIn"
            />
        </div>
    }
}

#[derive(Params, PartialEq, Clone, Debug)]
pub struct ContactParams {
    // Params isn't implemented for usize, only Option<usize>
    id: Option<usize>,
}

#[component]
pub fn Contact() -> impl IntoView {
    log!("rendering <Contact/>");

    log!(
        "ExampleContext should be Some(42). It is {:?}",
        use_context::<ExampleContext>()
    );

    on_cleanup(|| {
        log!("cleaning up <Contact/>");
    });

    let params = use_params::<ContactParams>();
    let contact = create_resource(
        move || {
            params
                .get()
                .map(|params| params.id.unwrap_or_default())
                .ok()
        },
        // any of the following would work (they're identical)
        // move |id| async move { get_contact(id).await }
        // move |id| get_contact(id),
        // get_contact
        get_contact,
    );

    create_effect(move |_| {
        log!("params = {:#?}", params.get());
    });

    let contact_display = move || match contact.get() {
        // None => loading, but will be caught by Suspense fallback
        // I'm only doing this explicitly for the example
        None => None,
        // Some(None) => has loaded and found no contact
        Some(None) => Some(
            view! { <p>"No contact with this ID was found."</p> }.into_any(),
        ),
        // Some(Some) => has loaded and found a contact
        Some(Some(contact)) => Some(
            view! {
                <section class="card">
                    <h1>{contact.first_name} " " {contact.last_name}</h1>
                    <p>{contact.address_1}<br/>{contact.address_2}</p>
                </section>
            }
            .into_any(),
        ),
    };

    view! {
        <div class="contact">
            <Transition fallback=move || view! {  <p>"Loading..."</p> }>
                {contact_display}
            </Transition>
        </div>
    }
}

#[component]
pub fn About() -> impl IntoView {
    log!("rendering <About/>");

    on_cleanup(|| {
        log!("cleaning up <About/>");
    });

    log!(
        "ExampleContext should be Some(0). It is {:?}",
        use_context::<ExampleContext>()
    );

    // use_navigate allows you to navigate programmatically by calling a function
    let navigate = use_navigate();

    view! {
        <>
            // note: this is just an illustration of how to use `use_navigate`
            // <button on:click> to navigate is an *anti-pattern*
            // you should ordinarily use a link instead,
            // both semantically and so your link will work before WASM loads
            <button on:click=move |_| navigate("/", Default::default())>
                "Home"
            </button>
            <h1>"About"</h1>
            <p>"Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat. Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur. Excepteur sint occaecat cupidatat non proident, sunt in culpa qui officia deserunt mollit anim id est laborum."</p>
        </>
    }
}

#[component]
pub fn Settings() -> impl IntoView {
    log!("rendering <Settings/>");

    on_cleanup(|| {
        log!("cleaning up <Settings/>");
    });

    view! {
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
