use super::*;
use ory_kratos_client::models::session::Session;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ViewableSession(pub Session);
impl IntoView for ViewableSession {
    fn into_view(self) -> View {
        format!("{:#?}", self).into_view()
    }
}

#[tracing::instrument]
#[server]
pub async fn session_who_am_i() -> Result<ViewableSession, ServerFnError> {
    use self::extractors::ExtractSession;
    let session = leptos_axum::extract::<ExtractSession>().await?.0;
    Ok(ViewableSession(session))
}

#[component]
pub fn HasSession() -> impl IntoView {
    let check_session = Action::<SessionWhoAmI, _>::server();
    view! {
        <button on:click=move|_|check_session.dispatch(SessionWhoAmI{})>
            Check Session Status
            <ErrorBoundary fallback=|errors|view!{<ErrorTemplate errors/>}>
                { move || check_session.value().get().map(|sesh|sesh.into_view()) }
            </ErrorBoundary>
        </button>
    }
}
