use super::*;
use ory_kratos_client::models::LoginFlow;
use ory_kratos_client::models::UiContainer;
use ory_kratos_client::models::UiText;
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ViewableLoginFlow(LoginFlow);
impl IntoView for ViewableLoginFlow {
    fn into_view(self) -> View {
        format!("{:?}", self).into_view()
    }
}
#[tracing::instrument]
#[server]
pub async fn init_login() -> Result<LoginResponse, ServerFnError> {
    let client = reqwest::ClientBuilder::new()
        .cookie_store(true)
        .redirect(reqwest::redirect::Policy::none())
        .build()?;
    // Get the csrf_token cookie.
    let resp = client
        .get("http://127.0.0.1:4433/self-service/login/browser")
        .send()
        .await?;
    let first_cookie = resp
        .cookies()
        .next()
        .ok_or(ServerFnError::new("Expecting a first cookie"))?;
    let csrf_token = first_cookie.value();
    let location = resp
        .headers()
        .get("Location")
        .ok_or(ServerFnError::new("expecting location in headers"))?
        .to_str()?;
    // Parses the url and takes first query which will be flow=FLOW_ID and we get FLOW_ID at .1
    let location_url = url::Url::parse(location)?;
    let id = location_url
        .query_pairs()
        .next()
        .ok_or(ServerFnError::new(
            "Expecting query in location header value",
        ))?
        .1;
    let set_cookie = resp
        .headers()
        .get("set-cookie")
        .ok_or(ServerFnError::new("expecting set-cookie in headers"))?
        .to_str()?;
    let flow = client
        .get("http://127.0.0.1:4433/self-service/login/flows")
        .query(&[("id", id)])
        .header("x-csrf-token", csrf_token)
        .send()
        .await?
        .json::<ViewableLoginFlow>()
        .await?;
    let opts = expect_context::<leptos_axum::ResponseOptions>();
    opts.append_header(
        axum::http::HeaderName::from_static("set-cookie"),
        axum::http::HeaderValue::from_str(set_cookie)?,
    );
    Ok(LoginResponse::Flow(flow))
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum LoginResponse {
    Flow(ViewableLoginFlow),
    Success,
}
impl IntoView for LoginResponse {
    fn into_view(self) -> View {
        match self {
            Self::Flow(view) => view.into_view(),
            _ => ().into_view(),
        }
    }
}

#[tracing::instrument]
#[server]
pub async fn login(mut body: HashMap<String, String>) -> Result<LoginResponse, ServerFnError> {
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
    let client = reqwest::ClientBuilder::new()
        .redirect(reqwest::redirect::Policy::none())
        .build()?;
    let resp = client
        .post(&action)
        .header("content-type", "application/json")
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
    if resp.status() == StatusCode::BAD_REQUEST {
        Ok(LoginResponse::Flow(resp.json::<ViewableLoginFlow>().await?))
    } else if resp.status() == StatusCode::OK {
        // ory_kratos_session cookie set above.
        Ok(LoginResponse::Success)
    } else if resp.status() == StatusCode::GONE {
        let err = resp.json::<GenericError>().await?;
        let err = format!("{:#?}", err);
        Err(ServerFnError::new(err))
    } else if resp.status() == StatusCode::UNPROCESSABLE_ENTITY {
        let err = resp.json::<ErrorBrowserLocationChangeRequired>().await?;
        let err = format!("{:#?}", err);
        Err(ServerFnError::new(err))
    } else if resp.status() == StatusCode::TEMPORARY_REDIRECT {
        let text = format!("{:#?}", resp);
        Err(ServerFnError::new(text))
    } else {
        // this is a status code that isn't covered by the documentation
        // https://www.ory.sh/docs/reference/api#tag/frontend/operation/updateLoginFlow
        let status_code = resp.status().as_u16();
        Err(ServerFnError::new(format!(
            "{status_code} is not covered under the ory documentation?"
        )))
    }
}

#[component]
pub fn LoginPage() -> impl IntoView {
    let login = Action::<Login, _>::server();
    let login_flow = create_local_resource(|| (), |_| async move { init_login().await });

    let login_resp = create_rw_signal(None::<Result<LoginResponse, ServerFnError>>);
    // after user tries to login we update the signal resp.
    create_effect(move |_| {
        if let Some(resp) = login.value().get() {
            login_resp.set(Some(resp))
        }
    });
    let login_flow = Signal::derive(move || {
        if let Some(resp) = login_resp.get() {
            Some(resp)
        } else {
            login_flow.get()
        }
    });
    let body = create_rw_signal(HashMap::new());
    view! {
      <Suspense fallback=||view!{Loading Login Details}>
        <ErrorBoundary fallback=|errors|view!{<ErrorTemplate errors/>}>
        {
          move ||
            login_flow.get().map(|resp|
                match resp {
                    Ok(resp) => {
                        match resp {
                            LoginResponse::Flow(ViewableLoginFlow(LoginFlow{ui:box UiContainer{nodes,action,messages,..},..})) => {
                                let form_inner_html = nodes.into_iter().map(|node|kratos_html(node,body)).collect_view();
                                body.update(move|map|{_=map.insert(String::from("action"),action);});
                                    view!{
                                        <form id=ids::LOGIN_FORM_ID
                                        on:submit=move|e|{
                                            e.prevent_default();
                                            e.stop_propagation();
                                            login.dispatch(Login{body:body.get_untracked()});
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
                            LoginResponse::Success => {
                                view!{<Redirect path="/"/>}.into_view()
                            }
                        }
                    }
                    err => err.into_view(),
                })
          }
        </ErrorBoundary>
      </Suspense>
    }
}
