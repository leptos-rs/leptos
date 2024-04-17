use axum::{async_trait, extract::FromRequestParts, RequestPartsExt};
use axum_extra::extract::CookieJar;
use http::request::Parts;
use ory_kratos_client::models::session::Session;
pub struct ExtractSession(pub Session);

#[async_trait]
impl<S> FromRequestParts<S> for ExtractSession
where
    S: Send + Sync,
{
    type Rejection = String;

    #[tracing::instrument(err(Debug),skip_all)]
    async fn from_request_parts(parts:&mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let cookie_jar = parts
            .extract::<CookieJar>()
            .await
            .unwrap();
        let csrf_cookie = cookie_jar
            .iter()
            .filter(|cookie| cookie.name().contains("csrf_token"))
            .next()
            .ok_or(
                "Expecting a csrf_token cookie to already be set if fetching a pre-existing flow".to_string()
            )?;
    let session_cookie = cookie_jar
        .get("ory_kratos_session")
        .ok_or("Ory Kratos Session cookie does not exist.".to_string())?;
    let client = reqwest::ClientBuilder::new()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();

    let resp = client
        .get("http://127.0.0.1:4433/sessions/whoami")
        .header("accept","application/json")
        .header(
            "cookie",
            format!("{}={}", csrf_cookie.name(), csrf_cookie.value()),
        )
        .header(
            "cookie",
            format!("{}={}",session_cookie.name(),session_cookie.value())
        )
        .send()
        .await
        .map_err(|err|format!("Error sending resp to whoami err:{:#?}",err).to_string())?;
        let session = resp.json::<Session>().await
            .map_err(|err|format!("Error getting json from body err:{:#?}",err).to_string())?;
        Ok(Self(session))
    }
}

