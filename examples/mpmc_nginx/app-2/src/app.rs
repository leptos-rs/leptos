use crate::error_template::{AppError, ErrorTemplate};
use leptos::*;
use leptos_meta::*;
use leptos_router::*;

#[component]
pub fn App() -> impl IntoView {
    // Provides context that manages stylesheets, titles, meta tags, etc.
    provide_meta_context();

    view! {


        // injects a stylesheet into the document <head>
        // id=leptos means cargo-leptos will hot-reload this stylesheet
        <Stylesheet id="leptos" href="/pkg2/app-2.css"/>

        // sets the document title
        <Title text="Welcome to Leptos"/>

        // content for this welcome page
        <Router fallback=|| {
            let mut outside_errors = Errors::default();
            outside_errors.insert_with_default_key(AppError::NotFound);
            view! {
                <ErrorTemplate outside_errors/>
            }
            .into_view()
        }>
            <main>
                <Routes>
                    <Route path="app2" view=HomePage/>
                </Routes>
            </main>
        </Router>
    }
}

/// Renders the home page of your application.
#[component]
fn HomePage() -> impl IntoView {
    use shared_server::SharedServerFunction;
    use shared_server_2::SharedServerFunction2;

    let hello_1_action = Action::<SharedServerFunction,_>::server();
    let hello_2_action = Action::<SharedServerFunction2,_>::server();

    let value_1 = create_rw_signal(String::from("waiting for update from shared server."));
    let value_2 = create_rw_signal(String::from("waiting for update from shared server 2."));

    //let hello_2 = create_resource(move || (), shared_server_2::shared_server_function);
    create_effect(move|_|{if let Some(Ok(msg)) = hello_1_action.value().get(){value_1.set(msg)}});
    create_effect(move|_|{if let Some(Ok(msg)) = hello_2_action.value().get(){value_2.set(msg)}});

    view! {
        <h1> App 2</h1>
        <div> action response from server 1 </div>
        <button on:click=move|_|hello_1_action.dispatch(SharedServerFunction{})>request from shared server 1</button>
        {move || value_1.get()}
        <div> action response from server 2 </div>
        <button on:click=move|_|hello_2_action.dispatch(SharedServerFunction2{})>request from shared server 2</button>
        {move || value_2.get()}
    }
}

