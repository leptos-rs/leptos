use http::status::StatusCode;
use leptos::prelude::{FromServerFnError, ServerFnErrorErr};
use leptos::server_fn::codec::JsonEncoding;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error, Serialize, Deserialize)]
pub enum AppError {
    #[error("Not Found")]
    NotFound,
    #[error("Internal Server Error")]
    InternalServerError,
    #[error(transparent)]
    ServerFn(#[from] ServerFnErrorErr),
}

impl FromServerFnError for AppError {
    type Encoder = JsonEncoding;

    fn status_code(&self) -> StatusCode {
        match self {
            AppError::NotFound => StatusCode::NOT_FOUND,
            AppError::InternalServerError => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::ServerFn(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn from_server_fn_error(value: ServerFnErrorErr) -> Self {
        AppError::ServerFn(value)
    }
}
