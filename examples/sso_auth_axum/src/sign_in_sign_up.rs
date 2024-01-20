use super::*;

#[cfg(feature = "ssr")]
pub mod ssr_imports {
    pub use crate::{
        auth::{ssr_imports::SqlCsrfToken, User},
        state::AppState,
    };
    pub use oauth2::{
        reqwest::async_http_client, AuthorizationCode, CsrfToken, Scope,
        TokenResponse,
    };
    pub use serde_json::Value;
}

#[server]
pub async fn google_sso() -> Result<String, ServerFnError> {
    use crate::ssr_imports::*;
    use ssr_imports::*;

    let oauth_client = expect_context::<AppState>().client;
    let pool = pool()?;

    // We get the authorization URL and CSRF_TOKEN
    let (authorize_url, csrf_token) = oauth_client
        .authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new("openid".to_string()))
        .add_scope(Scope::new("email".to_string()))
        // required for google auth refresh token to be part of the response.
        .add_extra_param("access_type", "offline")
        .add_extra_param("prompt", "consent")
        .url();
    let url = authorize_url.to_string();
    leptos::logging::log!("{url:?}");
    // Store the CSRF_TOKEN in our sqlite db.
    sqlx::query("INSERT INTO csrf_tokens (csrf_token) VALUES (?)")
        .bind(csrf_token.secret())
        .execute(&pool)
        .await
        .map(|_| ())?;

    // Send the url to the client.
    Ok(url)
}

#[component]
pub fn SignIn() -> impl IntoView {
    let g_auth = Action::<GoogleSso, _>::server();

    create_effect(move |_| {
        if let Some(Ok(redirect)) = g_auth.value().get() {
            window().location().set_href(&redirect).unwrap();
        }
    });

    view! {
      <div style="
      display:flex;
      flex-direction: column;
      justify-content: center;
      align-items: center;
      ">
        <div> {"Sign Up Sign In"} </div>
        <button style="display:flex;"  on:click=move|_| g_auth.dispatch(GoogleSso{})>
        <svg style="width:2rem;" version="1.1" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 48 48" xmlns:xlink="http://www.w3.org/1999/xlink" style="display: block;">
          <path fill="#EA4335" d="M24 9.5c3.54 0 6.71 1.22 9.21 3.6l6.85-6.85C35.9 2.38 30.47 0 24 0 14.62 0 6.51 5.38 2.56 13.22l7.98 6.19C12.43 13.72 17.74 9.5 24 9.5z"></path>
          <path fill="#4285F4" d="M46.98 24.55c0-1.57-.15-3.09-.38-4.55H24v9.02h12.94c-.58 2.96-2.26 5.48-4.78 7.18l7.73 6c4.51-4.18 7.09-10.36 7.09-17.65z"></path>
          <path fill="#FBBC05" d="M10.53 28.59c-.48-1.45-.76-2.99-.76-4.59s.27-3.14.76-4.59l-7.98-6.19C.92 16.46 0 20.12 0 24c0 3.88.92 7.54 2.56 10.78l7.97-6.19z"></path>
          <path fill="#34A853" d="M24 48c6.48 0 11.93-2.13 15.89-5.81l-7.73-6c-2.15 1.45-4.92 2.3-8.16 2.3-6.26 0-11.57-4.22-13.47-9.91l-7.98 6.19C6.51 42.62 14.62 48 24 48z"></path>
          <path fill="none" d="M0 0h48v48H0z"></path>
        </svg>
        <span style="margin-left:0.5rem;">"Sign in with Google"</span>
        </button>
        </div>
    }
}

#[server]
pub async fn handle_g_auth_redirect(
    provided_csrf: String,
    code: String,
) -> Result<(String, u64), ServerFnError> {
    use crate::ssr_imports::*;
    use ssr_imports::*;

    let oauth_client = expect_context::<AppState>().client;
    let pool = pool()?;
    let auth_session = auth()?;
    // If there's no match we'll return an error.
    let _ = sqlx::query_as::<_, SqlCsrfToken>(
        "SELECT csrf_token FROM csrf_tokens WHERE csrf_token = ?",
    )
    .bind(provided_csrf)
    .fetch_one(&pool)
    .await
    .map_err(|err| ServerFnError::new(format!("CSRF_TOKEN error : {err:?}")))?;

    let token_response = oauth_client
        .exchange_code(AuthorizationCode::new(code.clone()))
        .request_async(async_http_client)
        .await?;
    leptos::logging::log!("{:?}", &token_response);
    let access_token = token_response.access_token().secret();
    let expires_in = token_response.expires_in().unwrap().as_secs();
    let refresh_secret = token_response.refresh_token().unwrap().secret();
    let user_info_url = "https://www.googleapis.com/oauth2/v3/userinfo";
    let client = reqwest::Client::new();
    let response = client
        .get(user_info_url)
        .bearer_auth(access_token)
        .send()
        .await?;

    let email = if response.status().is_success() {
        let response_json: Value = response.json().await?;
        leptos::logging::log!("{response_json:?}");
        response_json["email"]
            .as_str()
            .expect("email to parse to string")
            .to_string()
    } else {
        return Err(ServerFnError::new(format!(
            "Response from google has status of {}",
            response.status()
        )));
    };

    let user = if let Some(user) = User::get_from_email(&email, &pool).await {
        user
    } else {
        sqlx::query("INSERT INTO users (email) VALUES (?)")
            .bind(&email)
            .execute(&pool)
            .await?;
        User::get_from_email(&email, &pool).await.unwrap()
    };

    auth_session.login_user(user.id);

    sqlx::query("DELETE FROM google_tokens WHERE user_id == ?")
        .bind(user.id)
        .execute(&pool)
        .await?;

    sqlx::query(
        "INSERT INTO google_tokens (user_id,access_secret,refresh_secret) \
         VALUES (?,?,?)",
    )
    .bind(user.id)
    .bind(access_token)
    .bind(refresh_secret)
    .execute(&pool)
    .await?;

    Ok((user.email, expires_in as u64))
}

#[derive(Params, Debug, PartialEq, Clone)]
pub struct OAuthParams {
    pub code: Option<String>,
    pub state: Option<String>,
}

#[component]
pub fn HandleGAuth() -> impl IntoView {
    let handle_g_auth_redirect = Action::<HandleGAuthRedirect, _>::server();

    let query = use_query::<OAuthParams>();
    let navigate = leptos_router::use_navigate();
    let rw_email = expect_context::<Email>().0;
    let rw_expires_in = expect_context::<ExpiresIn>().0;
    create_effect(move |_| {
        if let Some(Ok((email, expires_in))) =
            handle_g_auth_redirect.value().get()
        {
            rw_email.set(Some(email));
            rw_expires_in.set(expires_in);
            navigate("/", NavigateOptions::default());
        }
    });

    create_effect(move |_| {
        if let Ok(OAuthParams { code, state }) = query.get_untracked() {
            handle_g_auth_redirect.dispatch(HandleGAuthRedirect {
                provided_csrf: state.unwrap(),
                code: code.unwrap(),
            });
        } else {
            leptos::logging::log!("error parsing oauth params");
        }
    });
    view! {}
}

#[server]
pub async fn logout() -> Result<(), ServerFnError> {
    use crate::ssr_imports::*;

    let auth = auth()?;
    auth.logout_user();
    leptos_axum::redirect("/");
    Ok(())
}

#[component]
pub fn LogOut() -> impl IntoView {
    let log_out = create_server_action::<Logout>();
    view! {
        <button on:click=move|_|log_out.dispatch(Logout{})>{"log out"}</button>
    }
}
