use std::rc::Rc;

use thiserror::Error;

#[derive(Error, Debug, Clone)]
pub enum RouterError {
    #[error("loader found no data at this path")]
    NoMatch(String),
    #[error("route was matched, but loader returned None")]
    NotFound(String),
    #[error("could not find parameter {0}")]
    MissingParam(String),
    #[error("failed to deserialize parameters")]
    Params(Rc<dyn std::error::Error + Send + Sync>),
}
