#![feature(box_patterns)]

use crate::error_template::{AppError, ErrorTemplate};

use leptos::*;
use leptos_meta::*;
use leptos_router::*;

pub mod auth;
#[cfg(feature = "ssr")]
pub mod database_calls;
pub mod error_template;
use auth::*;
pub mod posts;
pub use posts::*;
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub struct IsLoggedIn(RwSignal<bool>);

#[component]
pub fn App() -> impl IntoView {
    // Provides context that manages stylesheets, titles, meta tags, etc.
    provide_meta_context();

    view! {
        <Stylesheet id="leptos" href="/pkg/ory-auth-example.css"/>

        // sets the document title
        <Title text="Welcome to Leptos"/>

        // content for this welcome page
        <Router fallback=|| {
            let mut outside_errors = Errors::default();
            outside_errors.insert_with_default_key(AppError::NotFound);
            view! { <ErrorTemplate outside_errors/> }.into_view()
        }>
            <main>
                <Routes>
                    <Route path="" view=HomePage/>
                    <Route path=ids::REGISTER_ROUTE view=RegistrationPage/>
                    <Route path=ids::VERIFICATION_ROUTE view=VerificationPage/>
                    <Route path=ids::LOGIN_ROUTE view=LoginPage/>
                    <Route path=ids::KRATOS_ERROR_ROUTE view=KratosErrorPage/>
                    <Route path=ids::RECOVERY_ROUTE view=RecoveryPage/>
                    <Route path=ids::SETTINGS_ROUTE view=SettingsPage/>
                </Routes>
            </main>
        </Router>
    }
}

/// Renders the home page of your application.
#[component]
fn HomePage() -> impl IntoView {
    let clear_cookies = Action::<ClearCookies, _>::server();
    view! {
        <h1>"Welcome to Leptos!"</h1>
        <div>
            <a href=ids::REGISTER_ROUTE id=ids::REGISTER_BUTTON_ID>Register</a>
        </div>
        <div>
            <a href=ids::LOGIN_ROUTE id=ids::LOGIN_BUTTON_ID>"Login"</a>
        </div>
        <div>
            <LogoutButton/>
        </div>
        <div>
            <button id=ids::CLEAR_COOKIES_BUTTON_ID
            on:click=move|_|clear_cookies.dispatch(ClearCookies{})>Clear cookies </button>
        </div>
        <div>
            <HasSession/>
        </div>
        <div>
            <PostPage/>
        </div>
        <div>
            <a href=ids::RECOVERY_ROUTE id=ids::RECOVER_EMAIL_BUTTON_ID>"Recovery Email"</a>
        </div>
        <div>
            <a href=ids::SETTINGS_ROUTE>"Settings"</a>
        </div>
    }
}

#[cfg(feature = "ssr")]
pub async fn clear_cookies_inner() -> Result<(), ServerFnError> {
    let opts = expect_context::<leptos_axum::ResponseOptions>();

    let cookie_jar = leptos_axum::extract::<axum_extra::extract::CookieJar>().await?;
    for cookie in cookie_jar.iter() {
        let mut cookie = cookie.clone();
        cookie.set_expires(
            time::OffsetDateTime::now_utc()
                .checked_sub(time::Duration::hours(24 * 356 * 10))
                .unwrap(),
        );
        cookie.set_max_age(time::Duration::seconds(0));
        cookie.set_path("/");
        // To clear an http only cookie, one must set an http only cookie.
        cookie.set_http_only(true);
        cookie.set_secure(true);
        let cookie = cookie.to_string();
        opts.append_header(
            axum::http::HeaderName::from_static("set-cookie"),
            axum::http::HeaderValue::from_str(&cookie)?,
        );
    }
    Ok(())
}

#[tracing::instrument(ret)]
#[server]
pub async fn clear_cookies() -> Result<(), ServerFnError> {
    clear_cookies_inner().await?;
    Ok(())
}
