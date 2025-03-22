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

type PartAlias<T> = Result<T, CustomError>;

#[server]
pub async fn part_alias_result() -> PartAlias<String> {
    Ok("hello".to_string())
}

fn main() {}