use std::{error, fmt, ops};

/// This is a result type into which any error can be converted,
/// and which can be used directly in your `view`.
///
/// All errors will be stored as [`struct@Error`].
pub type Result<T> = core::result::Result<T, Error>;

/// A generic wrapper for any error.
#[derive(Debug)]
#[repr(transparent)]
pub struct Error(Box<dyn error::Error + Send + Sync>);

impl ops::Deref for Error {
    type Target = Box<dyn error::Error + Send + Sync>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl<T> From<T> for Error
where
    T: error::Error + Send + Sync + 'static,
{
    fn from(value: T) -> Self {
        Error(Box::new(value))
    }
}
