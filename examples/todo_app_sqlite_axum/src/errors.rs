use miette::Diagnostic;
use thiserror::Error;

#[derive(Debug, Diagnostic, Error)]
pub enum TodoAppError {
    #[error("Not Found")]
    #[diagnostic(code("404"))]
    NotFound,
}
