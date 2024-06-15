use api_boundary as json;
use axum::{
    extract::State,
    http::Method,
    response::Json,
    routing::{get, post},
    Router,
};
use axum_extra::TypedHeader;
use headers::{authorization::Bearer, Authorization};
use parking_lot::RwLock;
use std::{env, net::SocketAddr, sync::Arc};
use tokio::net::TcpListener;
use tower_http::cors::{Any, CorsLayer};

mod adapters;
mod application;

use self::application::*;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    if let Err(err) = env::var("RUST_LOG") {
        match err {
            env::VarError::NotPresent => {
                env::set_var("RUST_LOG", "debug");
            }
            env::VarError::NotUnicode(_) => {
                return Err(anyhow::anyhow!(
                    "The value of 'RUST_LOG' does not contain valid unicode \
                     data."
                ));
            }
        }
    }
    env_logger::init();

    let shared_state = Arc::new(RwLock::new(AppState::default()));

    let cors_layer = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST])
        .allow_origin(Any);

    let app = Router::new()
        .route("/login", post(login))
        .route("/logout", post(logout))
        .route("/users", post(create_user))
        .route("/users", get(get_user_info))
        .route_layer(cors_layer)
        .with_state(shared_state);

    let addr = "0.0.0.0:3000".parse::<SocketAddr>()?;
    log::info!("Start listening on http://{addr}");
    let listener = TcpListener::bind(addr).await?;
    axum::serve(listener, app.into_make_service()).await?;
    Ok(())
}

type Result<T> = std::result::Result<Json<T>, Error>;

/// API error
#[derive(thiserror::Error, Debug)]
#[non_exhaustive]
enum Error {
    #[error(transparent)]
    CreateUser(#[from] CreateUserError),
    #[error(transparent)]
    Login(#[from] LoginError),
    #[error(transparent)]
    Logout(#[from] LogoutError),
    #[error(transparent)]
    Auth(#[from] AuthError),
    #[error(transparent)]
    Credentials(#[from] adapters::CredentialParsingError),
}

async fn create_user(
    State(state): State<Arc<RwLock<AppState>>>,
    Json(credentials): Json<json::Credentials>,
) -> Result<()> {
    let credentials = Credentials::try_from(credentials)?;
    state.write().create_user(credentials)?;
    Ok(Json(()))
}

async fn login(
    State(state): State<Arc<RwLock<AppState>>>,
    Json(credentials): Json<json::Credentials>,
) -> Result<json::ApiToken> {
    let json::Credentials { email, password } = credentials;
    log::debug!("{email} tries to login");
    let email = email.parse().map_err(|_|
        // Here we don't want to leak detailed info.
        LoginError::InvalidEmailOrPassword)?;
    let token = state
        .write()
        .login(email, &password)
        .map(|s| s.to_string())?;
    Ok(Json(json::ApiToken { token }))
}

async fn logout(
    State(state): State<Arc<RwLock<AppState>>>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
) -> Result<()> {
    state.write().logout(auth.token())?;
    Ok(Json(()))
}

async fn get_user_info(
    State(state): State<Arc<RwLock<AppState>>>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
) -> Result<json::UserInfo> {
    let user = state.read().authorize_user(auth.token())?;
    let CurrentUser { email, .. } = user;
    Ok(Json(json::UserInfo {
        email: email.into_string(),
    }))
}
