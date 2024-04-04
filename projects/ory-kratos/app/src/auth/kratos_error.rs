use super::*;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct KratosError {
    code: Option<usize>,
    message: Option<String>,
    reason: Option<String>,
    debug: Option<String>,
}

impl KratosError {
    pub fn to_err_msg(self) -> String {
        format!(
            "{}\n{}\n{}\n{}\n",
            self.code
                .map(|code| code.to_string())
                .unwrap_or("No Code included in error message".to_string()),
            self.message
                .unwrap_or("No message in Kratos Error".to_string()),
            self.reason
                .unwrap_or("No reason included in Kratos Error".to_string()),
            self.debug
                .unwrap_or("No debug included in Kratos Error".to_string())
        )
    }
}

impl IntoView for KratosError {
    fn into_view(self) -> View {
        view!{
            <div>{self.code.map(|code|code.to_string()).unwrap_or("No Code included in error message".to_string())}</div>
            <div>{self.message.unwrap_or("No message in Kratos Error".to_string())}</div>
            <div>{self.reason.unwrap_or("No reason included in Kratos Error".to_string())}</div>
            <div>{self.debug.unwrap_or("No debug included in Kratos Error".to_string())}</div>
        }.into_view()
    }
}

#[server]
pub async fn fetch_error(id: String) -> Result<KratosError, ServerFnError> {
    use ory_kratos_client::models::flow_error::FlowError;

    let client = reqwest::ClientBuilder::new()
        .redirect(reqwest::redirect::Policy::none())
        .build()?;
    //https://www.ory.sh/docs/kratos/self-service/flows/user-facing-errors
    let flow_error = client
        .get("http://127.0.0.1:4433/self-service/errors")
        .query(&[("id", id)])
        .send()
        .await?
        .json::<FlowError>()
        .await?;

    let error = flow_error.error.ok_or(ServerFnError::new(
        "Flow error does not contain an actual error. This is a server error.",
    ))?;
    Ok(serde_json::from_value::<KratosError>(error)?)
}

#[component]
pub fn KratosErrorPage() -> impl IntoView {
    let id = move || use_query_map().get().get("id").cloned().unwrap_or_default();
    let fetch_error_resource = create_resource(move || id(), |id| fetch_error(id));
    view! {
        <Suspense fallback=||"Error loading...".into_view()>
            <ErrorBoundary fallback=|errors|view!{<ErrorTemplate errors/>}>
            { move ||
                fetch_error_resource.get().map(|resp| match resp {
                    // kratos error isn't an error type, it's just a ui/data representation of a kratos error.
                    Ok(kratos_error) => kratos_error.into_view(),
                    // notice how we don't deconstruct i.e Err(err), this will bounce up to the error boundary
                    server_error => server_error.into_view()
                })
            }
            </ErrorBoundary>
        </Suspense>
    }
}
