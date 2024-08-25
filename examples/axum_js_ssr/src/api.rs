use leptos::{prelude::ServerFnError, server};

#[server]
pub async fn fetch_code() -> Result<String, ServerFnError> {
    // emulate loading of code from a database/version control/etc
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    Ok(crate::consts::CH05_02A.to_string())
}
