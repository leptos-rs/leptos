pub mod auth;
pub mod error_template;
#[cfg(feature = "ssr")]
pub mod fallback;
pub mod sign_in_sign_up;
#[cfg(feature = "ssr")]
pub mod state;
use leptos::{leptos_dom::helpers::TimeoutHandle, *};
use leptos_meta::*;
use leptos_router::*;
use sign_in_sign_up::*;

#[cfg(feature = "ssr")]
mod ssr_imports {
    pub use crate::auth::ssr_imports::{AuthSession, SqlRefreshToken};
    pub use leptos::{use_context, ServerFnError};
    pub use oauth2::{reqwest::async_http_client, TokenResponse};
    pub use sqlx::SqlitePool;

    pub fn pool() -> Result<SqlitePool, ServerFnError> {
        use_context::<SqlitePool>()
            .ok_or_else(|| ServerFnError::new("Pool missing."))
    }

    pub fn auth() -> Result<AuthSession, ServerFnError> {
        use_context::<AuthSession>()
            .ok_or_else(|| ServerFnError::new("Auth session missing."))
    }
}

#[derive(Clone, Debug)]
pub struct Email(RwSignal<Option<String>>);
#[derive(Clone, Debug)]
pub struct ExpiresIn(RwSignal<u64>);
#[server]
pub async fn refresh_token(email: String) -> Result<u64, ServerFnError> {
    use crate::{auth::User, state::AppState};
    use ssr_imports::*;

    let pool = pool()?;
    let oauth_client = expect_context::<AppState>().client;
    let user = User::get_from_email(&email, &pool)
        .await
        .ok_or(ServerFnError::new("User not found"))?;

    let refresh_secret = sqlx::query_as::<_, SqlRefreshToken>(
        "SELECT secret FROM google_refresh_tokens WHERE user_id = ?",
    )
    .bind(user.id)
    .fetch_one(&pool)
    .await?
    .secret;

    let token_response = oauth_client
        .exchange_refresh_token(&oauth2::RefreshToken::new(refresh_secret))
        .request_async(async_http_client)
        .await?;

    let access_token = token_response.access_token().secret();
    let expires_in = token_response.expires_in().unwrap().as_secs();
    let refresh_secret = token_response.refresh_token().unwrap().secret();
    sqlx::query("DELETE FROM google_tokens WHERE user_id == ?")
        .bind(user.id)
        .execute(&pool)
        .await?;
    sqlx::query(
        "INSERT OR REPLACE INTO google_tokens (user_id,access_secret,refresh_secret) \
         VALUES (?,?,?)",
    )
    .bind(user.id)
    .bind(access_token)
    .bind(refresh_secret)
    .execute(&pool)
    .await?;
    Ok(expires_in)
}

#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();
    let email = RwSignal::new(None::<String>);
    let rw_expires_in = RwSignal::new(0);
    provide_context(Email(email));
    provide_context(ExpiresIn(rw_expires_in));

    let display_email =
        move || email.get().unwrap_or(String::from("No email to display"));
    let refresh_token = create_server_action::<RefreshToken>();

    create_effect(move |handle: Option<Option<TimeoutHandle>>| {
        // If this effect is called, try to cancel the previous handle.
        if let Some(prev_handle) = handle.flatten() {
            prev_handle.clear();
        };
        // if expires_in isn't 0, then set a timeout that rerfresh a minute short of the refresh.
        let expires_in = rw_expires_in.get();
        if expires_in != 0 && email.get_untracked().is_some() {
            let handle = set_timeout_with_handle(
                move || {
                    refresh_token.dispatch(RefreshToken {
                        email: email.get_untracked().unwrap(),
                    })
                },
                std::time::Duration::from_secs(
                    // Google tokens last 3599 seconds, so we'll get a refresh token every 14 seconds.
                    expires_in.checked_sub(3545).unwrap_or_default(),
                ),
            )
            .unwrap();
            Some(handle)
        } else {
            None
        }
    });

    create_effect(move |_| {
        if let Some(Ok(expires_in)) = refresh_token.value().get() {
            rw_expires_in.set(expires_in);
        }
    });

    view! {
        <Stylesheet id="leptos" href="/pkg/sso_auth_axum.css"/>
        <Link rel="shortcut icon" type_="image/ico" href="/favicon.ico"/>
        <Title text="SSO Auth Axum"/>
        <Router>
            <main>
                <Routes>
                    <Route path="" view=move || {
                        view!{
                            {display_email}
                            <Show when=move || email.get().is_some() fallback=||view!{<SignIn/>}>
                                <LogOut/>
                            </Show>
                            }
                        }/>
                    <Route path="g_auth" view=||view!{<HandleGAuth/>}/>
                </Routes>
            </main>
        </Router>
    }
}

#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    _ = console_log::init_with_level(log::Level::Debug);
    console_error_panic_hook::set_once();

    leptos::mount_to_body(App);
}
