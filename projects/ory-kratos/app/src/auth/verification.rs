use std::collections::HashMap;

use super::*;
use ory_kratos_client::models::{UiContainer, UiText, VerificationFlow};
#[cfg(feature = "ssr")]
use tracing::debug;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ViewableVerificationFlow(VerificationFlow);
impl IntoView for ViewableVerificationFlow {
    fn into_view(self) -> View {
        format!("{:#?}", self.0).into_view()
    }
}
// https://{project}.projects.oryapis.com/self-service/verification/flows?id={}
#[tracing::instrument]
#[server]
pub async fn init_verification(
    flow_id: String,
) -> Result<Option<ViewableVerificationFlow>, ServerFnError> {
    let cookie_jar = leptos_axum::extract::<axum_extra::extract::CookieJar>().await?;
    let csrf_cookie = cookie_jar
        .iter()
        .filter(|cookie| cookie.name().contains("csrf_token"))
        .next()
        .ok_or(ServerFnError::new(
            "Expecting a csrf_token cookie to already be set if fetching a pre-existing flow",
        ))?;
    let client = reqwest::ClientBuilder::new()
        .redirect(reqwest::redirect::Policy::none())
        .build()?;
    // https://www.ory.sh/docs/reference/api#tag/frontend/operation/getVerificationFlow
    let resp = client
        .get("http://127.0.0.1:4433/self-service/verification/flows")
        .query(&[("id", flow_id)])
        //.header("x-csrf-token", csrf_token)
        //.header("content-type","application/json")
        .header(
            "cookie",
            format!("{}={}", csrf_cookie.name(), csrf_cookie.value()),
        )
        .send()
        .await?;
    if resp.status().as_u16() == 403 {
        debug!("{:#?}", resp.text().await?);
        Ok(None)
    } else {
        let flow = resp.json::<ViewableVerificationFlow>().await?;
        Ok(Some(flow))
    }
}
// verification flow complete POST
//http://127.0.0.1:4433/self-service/verification
#[tracing::instrument]
#[server]
pub async fn verify(
    mut body: HashMap<String, String>,
) -> Result<Option<ViewableVerificationFlow>, ServerFnError> {
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
    let client = reqwest::ClientBuilder::new()
        .redirect(reqwest::redirect::Policy::none())
        .build()?;
    let resp = client
        .post(&action)
        .header("accept", "application/json")
        .header(
            "cookie",
            format!("{}={}", csrf_cookie.name(), csrf_cookie.value()),
        )
        .json(&body)
        .send()
        .await?;

    let opts = expect_context::<leptos_axum::ResponseOptions>();
    opts.insert_header(
        axum::http::HeaderName::from_static("cache-control"),
        axum::http::HeaderValue::from_str("private, no-cache, no-store, must-revalidate")?,
    );
    match resp.json::<ViewableVerificationFlow>().await {
        Ok(flow) => Ok(Some(flow)),
        Err(_err) => Ok(None),
    }
}

#[component]
pub fn VerificationPage() -> impl IntoView {
    let verify = Action::<Verify, _>::server();

    let params_map = use_query_map();
    let init_verification = create_local_resource(
        move || params_map().get("flow").cloned().unwrap_or_default(),
        |flow_id| async move { init_verification(flow_id).await },
    );
    let verfication_resp =
        create_rw_signal(None::<Result<Option<ViewableVerificationFlow>, ServerFnError>>);
    create_effect(move |_| {
        if let Some(resp) = verify.value().get() {
            verfication_resp.set(Some(resp))
        }
    });
    let verification_flow = Signal::derive(move || {
        if let Some(flow) = verfication_resp.get() {
            Some(flow)
        } else {
            init_verification.get()
        }
    });
    let body = create_rw_signal(HashMap::new());
    view! {
        <Suspense fallback=||view!{Loading Verification Details}>
        <ErrorBoundary fallback=|errors|format!("ERRORS: {:?}",errors.get_untracked()).into_view()>
        {
          move ||
          verification_flow.get().map(|resp|{
                match resp {
                    Ok(Some(ViewableVerificationFlow(VerificationFlow{ui:box UiContainer{nodes,messages,action,..},..}))) => {
                            let form_inner_html = nodes.into_iter().map(|node|kratos_html(node,body)).collect_view();
                            body.update(|map|{_=map.insert(String::from("action"),action);});
                            view!{
                                <form on:submit=move|e|{
                                    e.prevent_default();
                                    e.stop_propagation();
                                    verify.dispatch(Verify{body:body.get_untracked()});
                                }
                                id=ids::VERIFICATION_FORM_ID
                                >
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
                }
            })
          }
        </ErrorBoundary>
      </Suspense>
    }
}
