use leptos::{logging, prelude::*};
use leptos_router::{
    components::{FlatRoutes, Route, Router},
    StaticSegment,
};

#[component]
pub fn App() -> impl IntoView {
    view! {
        // content for this welcome page
        <Router>
            <main id="app">
                <FlatRoutes fallback=NotFound>
                    <Route path=StaticSegment("") view=HomePage/>
                </FlatRoutes>
            </main>
        </Router>
    }
}

#[server]
async fn do_something(
    should_error: Option<String>,
) -> Result<String, ServerFnError> {
    if should_error.is_none() {
        Ok(String::from("Successful submit"))
    } else {
        Err(ServerFnError::ServerError(String::from(
            "You got an error!",
        )))
    }
}

/// Renders the home page of your application.
#[component]
fn HomePage() -> impl IntoView {
    let do_something_action = ServerAction::<DoSomething>::new();
    let value = Signal::derive(move || {
        do_something_action
            .value()
            .get()
            .unwrap_or_else(|| Ok(String::new()))
    });

    Effect::new_isomorphic(move |_| {
        logging::log!("Got value = {:?}", value.get());
    });

    view! {
        <h1>"Test the action form!"</h1>
        <ErrorBoundary fallback=move |error| {
            move || format!("{:#?}", error.get())
        }>
            <pre>{value}</pre>
            <ActionForm action=do_something_action attr:class="form">
                <label>"Should error: "<input type="checkbox" name="should_error"/></label>
                <button type="submit">Submit</button>
            </ActionForm>
        </ErrorBoundary>
    }
}

/// 404 - Not Found
#[component]
fn NotFound() -> impl IntoView {
    // set an HTTP status code 404
    // this is feature gated because it can only be done during
    // initial server-side rendering
    // if you navigate to the 404 page subsequently, the status
    // code will not be set because there is not a new HTTP request
    // to the server
    #[cfg(feature = "ssr")]
    {
        // this can be done inline because it's synchronous
        // if it were async, we'd use a server function
        let resp = expect_context::<leptos_actix::ResponseOptions>();
        resp.set_status(actix_web::http::StatusCode::NOT_FOUND);
    }

    view! { <h1>"Not Found"</h1> }
}
