use http::status::StatusCode;
use thiserror::Error;

#[derive(Debug, Clone, Error)]
pub enum TodoAppError {
    #[error("Not Found")]
    NotFound,
    #[error("Internal Server Error")]
    InternalServerError,
}

impl TodoAppError {
    pub fn status_code(&self) -> StatusCode {
        match self {
            TodoAppError::NotFound => StatusCode::NOT_FOUND,
            TodoAppError::InternalServerError => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}
