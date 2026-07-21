use crate::{error_template::ErrorTemplate, errors::AppError};
use leptos::prelude::*;
use leptos_meta::*;
use leptos_router::{
    components::{Route, Router, Routes},
    StaticSegment,
};

#[server(CauseInternalServerError, "/api")]
pub async fn cause_internal_server_error() -> Result<(), AppError> {
    // fake API delay
    std::thread::sleep(std::time::Duration::from_millis(500));

    Err(AppError::ServerFn(ServerFnErrorErr::ServerError(
        "Generic Server Error".to_string(),
    )))
}

#[server(CauseNotFoundError, "/api")]
pub async fn cause_not_found_error() -> Result<(), AppError> {
    // fake API delay
    std::thread::sleep(std::time::Duration::from_millis(500));

    Err(AppError::NotFound)
}

pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                <AutoReload options=options.clone() />
                <HydrationScripts options/>
                <MetaTags/>
            </head>
            <body>
                <App/>
            </body>
        </html>
    }
}

#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();
    view! {
        <Link rel="shortcut icon" type_="image/ico" href="/favicon.ico"/>
        <Stylesheet id="leptos" href="/pkg/errors_axum.css"/>
        <Router>
            <header>
                <h1>"Error Examples:"</h1>
            </header>
            <main>
                <Routes fallback=|| {
                    let mut errors = Errors::default();
                    errors.insert_with_default_key(AppError::NotFound);
                    view! {
                        <ErrorTemplate errors/>
                    }
                    .into_view()
                }>
                    <Route path=StaticSegment("") view=ExampleErrors/>
                </Routes>
            </main>
        </Router>
    }
}

#[component]
pub fn ExampleErrors() -> impl IntoView {
    let generate_internal_error =
        ServerAction::<CauseInternalServerError>::new();

    let generate_not_found_error = ServerAction::<CauseNotFoundError>::new();

    view! {
        <p>
            "These links will load 404 pages since they do not exist. Verify with browser development tools: " <br/>
            <a href="/404">"This links to a page that does not exist"</a><br/>
            <a href="/404" target="_blank">"Same link, but in a new tab"</a>
        </p>
        <p>
            "This button will generate a 404 error. Check browser network tools."
        </p>
        <ActionForm action=generate_not_found_error>
            <input name="error1" type="submit" value="Generate Not Found Error"/>
        </ActionForm>
        <p>
            "After pressing this button check browser network tools. Can be used even when WASM is blocked:"
        </p>
        <ActionForm action=generate_internal_error>
            <input name="error1" type="submit" value="Generate Internal Server Error"/>
        </ActionForm>
        <p>"The following <div> will always contain an error and cause this page to produce status 500. Check browser dev tools. "</p>
        <div>
            // note that the error boundaries could be placed above in the Router or lower down
            // in a particular route. The generated errors on the entire page contribute to the
            // final status code sent by the server when producing ssr pages.
            <ErrorBoundary fallback=|errors| view!{ <ErrorTemplate errors/>}>
                <ReturnsError/>
            </ErrorBoundary>
        </div>
    }
}

#[component]
pub fn ReturnsError() -> impl IntoView {
    Err::<String, AppError>(AppError::InternalServerError)
}
