use leptos::*;

#[cfg(feature="ssr")]
#[derive(Clone)]
pub struct SharedServerState;


#[tracing::instrument]
#[server(prefix="/api_shared",endpoint="/a")]
pub async fn shared_server_function() -> Result<String,ServerFnError> {
    tracing::debug!("SHARED SERVER 1");

    let _ : axum::Extension<SharedServerState> = leptos_axum::extract().await?;
    Ok("This message is from the shared server.".to_string())
}

//http://127.0.0.1:3002/api/shared/shared_server_function
// No hydrate function on a server function only server.