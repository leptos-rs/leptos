use std::{error, fmt, ops, sync::Arc};

/// This is a result type into which any error can be converted.
///
/// Results are stored as [`Error`].
pub type Result<T, E = Error> = core::result::Result<T, E>;

/// A generic wrapper for any error.
#[derive(Debug, Clone)]
#[repr(transparent)]
pub struct Error(Arc<dyn error::Error + Send + Sync>);

impl Error {
    /// Converts the wrapper into the inner reference-counted error.
    pub fn into_inner(self) -> Arc<dyn error::Error + Send + Sync> {
        Arc::clone(&self.0)
    }
}

impl ops::Deref for Error {
    type Target = Arc<dyn error::Error + Send + Sync>;

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
        Error(Arc::new(value))
    }
}
