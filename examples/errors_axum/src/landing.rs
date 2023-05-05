use crate::{error_template::ErrorTemplate, errors::AppError};
use leptos::*;
use leptos_meta::*;
use leptos_router::*;

#[cfg(feature = "ssr")]
pub fn register_server_functions() {
    _ = CauseInternalServerError::register();
}

#[server(CauseInternalServerError, "/api")]
pub async fn cause_internal_server_error() -> Result<(), ServerFnError> {
    // fake API delay
    std::thread::sleep(std::time::Duration::from_millis(1250));

    Err(ServerFnError::ServerError(
        "Generic Server Error".to_string(),
    ))
}

#[component]
pub fn App(cx: Scope) -> impl IntoView {
    //let id = use_context::<String>(cx);
    provide_meta_context(cx);
    view! {
        cx,
        <Link rel="shortcut icon" type_="image/ico" href="/favicon.ico"/>
        <Stylesheet id="leptos" href="/pkg/errors_axum.css"/>
        <Router fallback=|cx| {
            let mut outside_errors = Errors::default();
            outside_errors.insert_with_default_key(AppError::NotFound);
            view! { cx,
                <ErrorTemplate outside_errors/>
            }
            .into_view(cx)
        }>
            <header>
                <h1>"Error Examples:"</h1>
            </header>
            <main>
                <Routes>
                    <Route path="" view=|cx| view! {
                        cx,
                        <ExampleErrors/>
                    }/>
                </Routes>
            </main>
        </Router>
    }
}

#[component]
pub fn ExampleErrors(cx: Scope) -> impl IntoView {
    let generate_internal_error =
        create_server_action::<CauseInternalServerError>(cx);

    view! { cx,
        <p>
            "These links will load 404 pages since they do not exist. Verify with browser development tools: " <br/>
            <a href="/404">"This links to a page that does not exist"</a><br/>
            <a href="/404" target="_blank">"Same link, but in a new tab"</a>
        </p>
        <p>
            "After pressing this button check browser network tools. Can be used even when WASM is blocked:"
            <ActionForm action=generate_internal_error>
                <input name="error1" type="submit" value="Generate Internal Server Error"/>
            </ActionForm>
        </p>
        <p>"The following <div> will always contain an error and cause this page to produce status 500. Check browser dev tools. "</p>
        <div>
        // note that the error boundaries could be placed above in the Router or lower down
        // in a particular route. The generated errors on the entire page contribute to the
        // final status code sent by the server when producing ssr pages.
        <ErrorBoundary fallback=|cx, errors| view!{cx, <ErrorTemplate errors=errors/>}>
            <ReturnsError/>
        </ErrorBoundary>
        </div>
    }
}

#[component]
pub fn ReturnsError(_cx: Scope) -> impl IntoView {
    Err::<String, AppError>(AppError::InternalServerError)
}
