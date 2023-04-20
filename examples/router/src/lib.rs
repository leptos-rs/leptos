mod api;
use crate::api::*;
use leptos::*;
use leptos_router::*;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
struct ExampleContext(i32);

#[component]
pub fn RouterExample(cx: Scope) -> impl IntoView {
    log::debug!("rendering <RouterExample/>");

    // contexts are passed down through the route tree
    provide_context(cx, ExampleContext(0));

    view! { cx,
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
                        view=move |cx| view! { cx,  <About/> }
                    />
                    <Route
                        path="settings"
                        view=move |cx| view! { cx,  <Settings/> }
                    />
                    <Route
                        path="redirect-home"
                        view=move |cx| view! { cx, <Redirect path="/"/> }
                    />
                </AnimatedRoutes>
            </main>
        </Router>
    }
}

// You can define other routes in their own component.
// Use a #[component(transparent)] that returns a <Route/>.
#[component(transparent)]
pub fn ContactRoutes(cx: Scope) -> impl IntoView {
    view! { cx,
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
    }
}

#[component]
pub fn ContactList(cx: Scope) -> impl IntoView {
    log::debug!("rendering <ContactList/>");

    // contexts are passed down through the route tree
    provide_context(cx, ExampleContext(42));

    on_cleanup(cx, || {
        log!("cleaning up <ContactList/>");
    });

    let location = use_location(cx);
    let contacts =
        create_resource(cx, move || location.search.get(), get_contacts);
    let contacts = move || {
        contacts.read(cx).map(|contacts| {
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
    id: usize,
}

#[component]
pub fn Contact(cx: Scope) -> impl IntoView {
    log::debug!("rendering <Contact/>");

    log::debug!(
        "ExampleContext should be Some(42). It is {:?}",
        use_context::<ExampleContext>(cx)
    );

    on_cleanup(cx, || {
        log!("cleaning up <Contact/>");
    });

    let params = use_params::<ContactParams>(cx);
    let contact = create_resource(
        cx,
        move || params().map(|params| params.id).ok(),
        // any of the following would work (they're identical)
        // move |id| async move { get_contact(id).await }
        // move |id| get_contact(id),
        // get_contact
        get_contact,
    );

    create_effect(cx, move |_| {
        log!("params = {:#?}", params.get());
    });

    let contact_display = move || match contact.read(cx) {
        // None => loading, but will be caught by Suspense fallback
        // I'm only doing this explicitly for the example
        None => None,
        // Some(None) => has loaded and found no contact
        Some(None) => Some(
            view! { cx, <p>"No contact with this ID was found."</p> }
                .into_any(),
        ),
        // Some(Some) => has loaded and found a contact
        Some(Some(contact)) => Some(
            view! { cx,
                <section class="card">
                    <h1>{contact.first_name} " " {contact.last_name}</h1>
                    <p>{contact.address_1}<br/>{contact.address_2}</p>
                </section>
            }
            .into_any(),
        ),
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

    on_cleanup(cx, || {
        log!("cleaning up <About/>");
    });

    log::debug!(
        "ExampleContext should be Some(0). It is {:?}",
        use_context::<ExampleContext>(cx)
    );

    // use_navigate allows you to navigate programmatically by calling a function
    let navigate = use_navigate(cx);

    view! { cx,
        <>
            // note: this is just an illustration of how to use `use_navigate`
            // <button on:click> to navigate is an *anti-pattern*
            // you should ordinarily use a link instead,
            // both semantically and so your link will work before WASM loads
            <button on:click=move |_| { _ = navigate("/", Default::default()); }>
                "Home"
            </button>
            <h1>"About"</h1>
            <p>"Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat. Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur. Excepteur sint occaecat cupidatat non proident, sunt in culpa qui officia deserunt mollit anim id est laborum."</p>
        </>
    }
}

#[component]
pub fn Settings(cx: Scope) -> impl IntoView {
    log::debug!("rendering <Settings/>");

    on_cleanup(cx, || {
        log!("cleaning up <Settings/>");
    });

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
