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

impl PartialEq for RouterError {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::NoMatch(l0), Self::NoMatch(r0)) => l0 == r0,
            (Self::NotFound(l0), Self::NotFound(r0)) => l0 == r0,
            (Self::MissingParam(l0), Self::MissingParam(r0)) => l0 == r0,
            (Self::Params(l0), Self::Params(r0)) => false,
            _ => false,
        }
    }
}
