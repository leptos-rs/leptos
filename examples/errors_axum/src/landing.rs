use crate::{
    error_template::{ErrorTemplate, ErrorTemplateProps},
    errors::AppError,
};
use cfg_if::cfg_if;
use leptos::*;
use leptos_meta::*;
use leptos_router::*;

cfg_if! { if #[cfg(feature = "ssr")] {
    pub fn register_server_functions() {
        _ = CauseInternalServerError::register();
        _ = CauseNotFoundError::register();
    }
}}

#[server(CauseInternalServerError, "/api")]
pub async fn cause_internal_server_error() -> Result<(), ServerFnError> {
    // fake API delay
    std::thread::sleep(std::time::Duration::from_millis(1250));

    Err(ServerFnError::ServerError(
        "Generic Server Error".to_string(),
    ))
}

#[server(CauseNotFoundError, "/api")]
pub async fn cause_not_found_error() -> Result<(), ServerFnError> {
    Err(ServerFnError::ServerError("Not Found".to_string()))
}

#[component]
pub fn App(cx: Scope) -> impl IntoView {
    //let id = use_context::<String>(cx);
    provide_meta_context(cx);
    view! {
        cx,
        <Link rel="shortcut icon" type_="image/ico" href="/favicon.ico"/>
        <Stylesheet id="leptos" href="/pkg/errors_axum.css"/>
        <Router>
            <header>
                <h1>"Error Examples:"</h1>
            </header>
            <main>
                <Routes>
                    <Route path="" view=|cx| view! {
                        cx,
                        <ErrorBoundary fallback=|cx, errors| view!{cx, <ErrorTemplate errors=errors/>}>
                            <ExampleErrors/>
                        </ErrorBoundary>
                    }/>
                </Routes>
            </main>
        </Router>
    }
}

#[component]
pub fn ExampleErrors(cx: Scope) -> impl IntoView {
    view! {
            cx,
            <p>
                "This link will load a 404 page since it does not exist. Verify with browser development tools:"
                <a href="/404">"This Page Does not Exist"</a>
            </p>
            <p>
                "The following <div> will always contain an error and cause the page to produce status 500. Check browser dev tools. "
            </p>
            <div>
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
