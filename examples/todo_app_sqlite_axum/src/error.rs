use thiserror::Error;

#[derive(Error, Debug)]
pub enum TodoAppError {
    #[error("An Error Occured")]
    AnError,
}
