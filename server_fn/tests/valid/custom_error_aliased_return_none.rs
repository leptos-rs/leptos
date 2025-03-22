use server_fn_macro_default::server;
use server_fn::error::{FromServerFnError, ServerFnErrorErr};

#[derive(Debug, thiserror::Error, Clone, serde::Serialize, serde::Deserialize)]
pub enum CustomError {
    #[error("error a")]
    ErrorA,
    #[error("error b")]
    ErrorB,
}

impl FromServerFnError for CustomError {
    fn from_server_fn_error(_: ServerFnErrorErr) -> Self {
        Self::ErrorA
    }
}

#[server]
pub async fn no_alias_result() -> Result<String, CustomError> {
    Ok("hello".to_string())
}

fn main() {}