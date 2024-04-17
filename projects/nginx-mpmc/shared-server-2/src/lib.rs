use leptos::*;

#[cfg(feature="ssr")]
#[derive(Clone)]
pub struct SharedServerState2;

#[tracing::instrument]
#[server(prefix="/api_shared2",endpoint="/a")]
pub async fn shared_server_function2() -> Result<String,ServerFnError> {
    tracing::debug!("SHARED SERVER 2");

    let _ : axum::Extension<SharedServerState2> = leptos_axum::extract().await?;
    Ok("This message is from the shared server 2.".to_string())
}

//http://127.0.0.1:3002/api/shared/shared_server_function
// No hydrate function on a server function only server.