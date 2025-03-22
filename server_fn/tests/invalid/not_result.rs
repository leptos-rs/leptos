use server_fn_macro_default::server;
use server_fn::error::{FromServerFnError, ServerFnErrorErr};

#[derive(Debug, thiserror::Error, Clone, serde::Serialize, serde::Deserialize)]
pub enum CustomError {
    #[error("error a")]
    A,
    #[error("error b")]
    B,
}

impl FromServerFnError for CustomError {
    fn from_server_fn_error(_: ServerFnErrorErr) -> Self {
        Self::A
    }
}

#[server]
pub async fn full_alias_result() -> CustomError {
    CustomError::A
}

fn main() {}