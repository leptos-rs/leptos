use super::kratos_html;
use super::*;
use ory_kratos_client::models::RegistrationFlow;
use ory_kratos_client::models::UiContainer;
use ory_kratos_client::models::UiText;
use std::collections::HashMap;

#[cfg(feature = "ssr")]
use reqwest::StatusCode;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ViewableRegistrationFlow(RegistrationFlow);
impl IntoView for ViewableRegistrationFlow {
    fn into_view(self) -> View {
        format!("{:?}", self).into_view()
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum RegistrationResponse {
    Flow(ViewableRegistrationFlow),
    Success,
}
impl IntoView for RegistrationResponse {
    fn into_view(self) -> View {
        match self {
            Self::Flow(view) => view.into_view(),
            _ => ().into_view(),
        }
    }
}
#[tracing::instrument]
#[server]
pub async fn init_registration() -> Result<RegistrationResponse, ServerFnError> {
    let client = reqwest::ClientBuilder::new()
        .cookie_store(true)
        .redirect(reqwest::redirect::Policy::none())
        .build()?;
    // Get the csrf_token cookie.
    let resp = client
        .get("http://127.0.0.1:4433/self-service/registration/browser")
        .send()
        .await?;
    let first_cookie = resp
        .cookies()
        .filter(|c| c.name().contains("csrf_token"))
        .next()
        .ok_or(ServerFnError::new(
            "Expecting a cookie with csrf_token in name",
        ))?;
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
    let resp = client
        .get("http://127.0.0.1:4433/self-service/registration/flows")
        .query(&[("id", id)])
        .header("x-csrf-token", csrf_token)
        .send()
        .await?;
    let flow = resp.json::<ViewableRegistrationFlow>().await?;
    let opts = expect_context::<leptos_axum::ResponseOptions>();
    opts.insert_header(
        axum::http::HeaderName::from_static("cache-control"),
        axum::http::HeaderValue::from_str("private, no-cache, no-store, must-revalidate")?,
    );
    opts.append_header(
        axum::http::HeaderName::from_static("set-cookie"),
        axum::http::HeaderValue::from_str(set_cookie)?,
    );
    Ok(RegistrationResponse::Flow(flow))
}

#[tracing::instrument(err)]
#[server]
pub async fn register(
    mut body: HashMap<String, String>,
) -> Result<RegistrationResponse, ServerFnError> {
    use ory_kratos_client::models::error_browser_location_change_required::ErrorBrowserLocationChangeRequired;
    use ory_kratos_client::models::generic_error::GenericError;
    use ory_kratos_client::models::successful_native_registration::SuccessfulNativeRegistration;

    let pool = leptos_axum::extract::<axum::Extension<sqlx::SqlitePool>>().await?;

    let action = body
        .remove("action")
        .ok_or(ServerFnError::new("Can't find action on body."))?;
    let email = body
        .get("traits.email")
        .cloned()
        .ok_or(ServerFnError::new("Can't find traits.email on body."))?;
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
        //.header("content-type", "application/json")
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
    for value in resp.headers().get_all("set-cookie").iter() {
        opts.append_header(
            axum::http::HeaderName::from_static("set-cookie"),
            axum::http::HeaderValue::from_str(value.to_str()?)?,
        );
    }
    if resp.status() == StatusCode::BAD_REQUEST {
        Ok(RegistrationResponse::Flow(
            resp.json::<ViewableRegistrationFlow>().await?,
        ))
    } else if resp.status() == StatusCode::OK {
        // get identity, session, session token
        let SuccessfulNativeRegistration { identity, .. } =
            resp.json::<SuccessfulNativeRegistration>().await?;
        let identity_id = identity.id;
        crate::database_calls::create_user(&pool, &identity_id, &email).await?;
        //discard all? what about session_token? I guess we aren't allowing logging in after registration without verification..
        Ok(RegistrationResponse::Success)
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
        // https://www.ory.sh/docs/reference/api#tag/frontend/operation/updateRegistrationFlow
        let status_code = resp.status().as_u16();
        Err(ServerFnError::new(format!(
            "{status_code} is not covered under the ory documentation?"
        )))
    }
}

#[component]
pub fn RegistrationPage() -> impl IntoView {
    let register = Action::<Register, _>::server();

    // when we hit the page initiate a flow with kratos and get back data for ui renering.
    let registration_flow =
        create_local_resource(|| (), |_| async move { init_registration().await });
    // Is none if user hasn't submitted data.
    let register_resp = create_rw_signal(None::<Result<RegistrationResponse, ServerFnError>>);
    // after user tries to register we update the signal resp.
    create_effect(move |_| {
        if let Some(resp) = register.value().get() {
            register_resp.set(Some(resp))
        }
    });
    // Merge our resource and our action results into a single signal.
    // if the user hasn't tried to register yet we'll render the initial flow.
    // if they have, we'll render the updated flow (including error messages etc).
    let registration_flow = Signal::derive(move || {
        if let Some(resp) = register_resp.get() {
            Some(resp)
        } else {
            registration_flow.get()
        }
    });
    // this is the body of our registration form, we don't know what the inputs are so it's a stand in for some
    // json map of unknown argument length with type of string.
    let body = create_rw_signal(HashMap::new());
    view! {
        // we'll render the fallback when the user hits the page for the first time
      <Suspense fallback=||view!{Loading Registration Details}>
        // if we get any errors, from either server functions we've merged we'll render them here.
        <ErrorBoundary fallback=|errors|view!{<ErrorTemplate errors/>}>
        {
          move ||
          // this is the resource XOR the results of the register action.
          registration_flow.get().map(|resp|{
                match resp {
                    // TODO add Oauth using the flow args (see type docs)
                    Ok(resp) => {
                        match resp {
                            RegistrationResponse::Flow(ViewableRegistrationFlow(RegistrationFlow{ui:box UiContainer{nodes,action,messages,..},..}))
                            => {
                                let form_inner_html = nodes.into_iter().map(|node|kratos_html(node,body)).collect_view();
                                body.update(move|map|{_=map.insert(String::from("action"),action);});

                                view!{
                                    <form

                                    on:submit=move|e|{
                                        e.prevent_default();
                                        e.stop_propagation();
                                        register.dispatch(Register{body:body.get_untracked()});
                                    }
                                    id=ids::REGISTRATION_FORM_ID
                                    >
                                    {form_inner_html}
                                    // kratos_html renders messages for each node and these are the messages attached to the entire form.
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
                        RegistrationResponse::Success => {
                            view!{<div id=ids::VERIFY_EMAIL_DIV_ID>"Check Email for Verification"</div>}.into_view()
                           }
                        }
                    },
                    err => err.into_view(),
                }
            })
          }
        </ErrorBoundary>
      </Suspense>
    }
}
