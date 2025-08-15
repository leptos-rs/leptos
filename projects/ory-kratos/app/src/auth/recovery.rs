use std::collections::HashMap;

use super::*;
use ory_kratos_client::models::{
    ContinueWith, ContinueWithSettingsUiFlow, ErrorGeneric, RecoveryFlow, UiContainer, UiText,
};
/*
    User clicks recover account button and is directed to the initiate recovery page
    On the initiate recovery page they are asked for their email
    We send an email to them with a recovery code to recover the identity
    and a link to the recovery page which will prompt them for the code.
    We validate the code
    and we then direct them to the settings page for them to change their password.
*/

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ViewableRecoveryFlow(RecoveryFlow);
// Implment IntoView, not because we want to use IntoView - but, just so we can use ErrorBoundary on the error.
impl IntoView for ViewableRecoveryFlow {
    fn into_view(self) -> View {
        format!("{:?}", self).into_view()
    }
}

pub struct ViewableContinueWith(pub Vec<ContinueWith>);
impl IntoView for ViewableContinueWith {
    fn into_view(self) -> View {
        if let Some(first) = self.0.first() {
            match first {
                ContinueWith::ContinueWithSetOrySessionToken { ory_session_token } => todo!(),
                ContinueWith::ContinueWithRecoveryUi { flow } => todo!(),
                ContinueWith::ContinueWithSettingsUi {
                    flow: box ContinueWithSettingsUiFlow { id },
                } => view! {<Redirect path=format!("/settings?flow={id}")/>}.into_view(),
                ContinueWith::ContinueWithVerificationUi { flow } => todo!(),
            }
        } else {
            ().into_view()
        }
    }
}
#[tracing::instrument]
#[server]
pub async fn init_recovery_flow() -> Result<ViewableRecoveryFlow, ServerFnError> {
    let client = reqwest::ClientBuilder::new()
        .cookie_store(true)
        .redirect(reqwest::redirect::Policy::none())
        .build()?;
    // Get the csrf_token cookie.
    let resp = client
        .get("http://127.0.0.1:4433/self-service/recovery/browser")
        .header("accept", "application/json")
        .send()
        .await?;

    let cookie = resp
        .headers()
        .get("set-cookie")
        .ok_or(ServerFnError::new("Expecting a cookie"))?
        .to_str()?;
    let opts = expect_context::<leptos_axum::ResponseOptions>();
    opts.append_header(
        axum::http::HeaderName::from_static("set-cookie"),
        axum::http::HeaderValue::from_str(cookie)?,
    );
    let status = resp.status();
    if status == reqwest::StatusCode::OK {
        let flow = resp.json::<RecoveryFlow>().await?;
        Ok(ViewableRecoveryFlow(flow))
    } else if status == reqwest::StatusCode::BAD_REQUEST {
        let error = resp.json::<ErrorGeneric>().await?;
        Err(ServerFnError::new(format!("{error:#?}")))
    } else {
        tracing::error!(
            " UNHANDLED STATUS: {} \n text: {}",
            status,
            resp.text().await?
        );
        Err(ServerFnError::new("Developer made an oopsies."))
    }
}

#[tracing::instrument(ret)]
#[server]
pub async fn process_recovery(
    mut body: HashMap<String, String>,
) -> Result<ViewableRecoveryFlow, ServerFnError> {
    use ory_kratos_client::models::error_browser_location_change_required::ErrorBrowserLocationChangeRequired;
    use ory_kratos_client::models::generic_error::GenericError;
    use reqwest::StatusCode;

    let action = body
        .remove("action")
        .ok_or(ServerFnError::new("Can't find action on body."))?;
    let cookie_jar = leptos_axum::extract::<axum_extra::extract::CookieJar>().await?;
    let csrf_cookie = cookie_jar
        .iter()
        .filter(|cookie| cookie.name().contains("csrf_token"))
        .next()
        .ok_or(ServerFnError::new(
            "Expecting a csrf_token cookie to already be set if fetching a pre-existing flow",
        ))?;
    let csrf_token = csrf_cookie.value();
    let client = reqwest::ClientBuilder::new()
        .redirect(reqwest::redirect::Policy::none())
        .build()?;
    let resp = client
        .post(&action)
        .header("x-csrf-token", csrf_token)
        .header("content-type", "application/json")
        .header("accept", "application/json")
        .header(
            "cookie",
            format!("{}={}", csrf_cookie.name(), csrf_cookie.value()),
        )
        .body(serde_json::to_string(&body)?)
        .send()
        .await?;

    let opts = expect_context::<leptos_axum::ResponseOptions>();
    opts.insert_header(
        axum::http::HeaderName::from_static("cache-control"),
        axum::http::HeaderValue::from_str("private, no-cache, no-store, must-revalidate")?,
    );
    for value in resp.headers().get_all("set-cookie").iter() {
        opts.append_header(
            axum::http::HeaderName::from_static("set-cookie"),
            axum::http::HeaderValue::from_str(value.to_str()?)?,
        );
    }
    if resp.status() == StatusCode::BAD_REQUEST || resp.status() == StatusCode::OK {
        Ok(resp.json::<ViewableRecoveryFlow>().await?)
    } else if resp.status() == StatusCode::SEE_OTHER {
        let see_response = format!("{resp:#?}");
        let resp_text = resp.text().await?;
        let err = format!("Developer needs to handle 303 SEE OTHER resp : \n  {see_response} \n body: \n {resp_text}");
        Err(ServerFnError::new(err))
    } else if resp.status() == StatusCode::GONE {
        let err = resp.json::<GenericError>().await?;
        let err = format!("{:#?}", err);
        Err(ServerFnError::new(err))
    } else if resp.status() == StatusCode::UNPROCESSABLE_ENTITY {
        let err = resp.json::<ErrorBrowserLocationChangeRequired>().await?;
        let err = format!("{:#?}", err);
        Err(ServerFnError::new(err))
    } else {
        // this is a status code that isn't covered by the documentation
        // https://www.ory.sh/docs/reference/api#tag/frontend/operation/updateRecoveryFlow
        let status_code = resp.status().as_u16();
        Err(ServerFnError::new(format!(
            "{status_code} is not covered under the ory documentation?"
        )))
    }
}

#[component]
pub fn RecoveryPage() -> impl IntoView {
    let recovery_flow = create_local_resource(|| (), |_| init_recovery_flow());
    let recovery = Action::<ProcessRecovery, _>::server();

    let recovery_resp = create_rw_signal(None::<Result<ViewableRecoveryFlow, ServerFnError>>);
    create_effect(move |_| {
        if let Some(resp) = recovery.value().get() {
            recovery_resp.set(Some(resp))
        }
    });
    let recovery_flow = Signal::derive(move || {
        if let Some(resp) = recovery_resp.get() {
            Some(resp)
        } else {
            recovery_flow.get()
        }
    });
    let body = create_rw_signal(HashMap::new());
    view! {
        <Suspense fallback=||view!{}>
            <ErrorBoundary fallback=|errors|view!{<ErrorTemplate errors/>}>
            {
                move ||
                recovery_flow.get().map(|resp|
                      match resp {
                        Ok(ViewableRecoveryFlow(RecoveryFlow{
                            continue_with,
                            ui:box UiContainer{nodes,action,messages,..},..})) => {
                                if let Some(continue_with) = continue_with {
                                    return ViewableContinueWith(continue_with).into_view();
                                }
                            let form_inner_html = nodes.into_iter().map(|node|kratos_html(node,body)).collect_view();
                            body.update(move|map|{_=map.insert(String::from("action"),action);});
                                view!{
                                    <form id=ids::RECOVERY_FORM_ID
                                    on:submit=move|e|{
                                        if body.get().get(&String::from("code")).is_some() {
                                            // if we have a code we need to drop the email which will be stored from earlier.
                                            // if we include the email then ory kratos server will not try to validate the code.
                                            // but instead send another recovery email.
                                            body.update(move|map|{_=map.remove(&String::from("email"));});
                                        }
                                        e.prevent_default();
                                        e.stop_propagation();
                                        recovery.dispatch(ProcessRecovery{body:body.get_untracked()});
                                    }>
                                    {form_inner_html}
                                    {messages.map(|messages|{
                                        view!{
                                            <For
                                                each=move || messages.clone().into_iter()
                                                key=|text| text.id
                                                children=move |text: UiText| {
                                                  view! {
                                                    <p id=text.id>{text.text}</p>
                                                  }
                                                }
                                            />
                                        }
                                    }).unwrap_or_default()}
                                    </form>
                                }.into_view()
                          },
                          err => err.into_view(),
                      })
                }
            </ErrorBoundary>
        </Suspense>
    }
}
