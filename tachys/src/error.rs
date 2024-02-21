use std::{error, fmt, ops, result};

/// This is a result type into which any error can be converted,
/// and which can be used directly in your `view`.
///
/// All errors will be stored as [`struct@AnyError`].
pub type Result<T> = result::Result<T, AnyError>;

/// A generic wrapper for any error.
#[derive(Debug)]
#[repr(transparent)]
pub struct AnyError(Box<dyn error::Error>);

impl AnyError {
    pub fn new(err: impl error::Error + 'static) -> Self {
        Self(Box::new(err))
    }
}

impl ops::Deref for AnyError {
    type Target = Box<dyn error::Error>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl fmt::Display for AnyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl error::Error for AnyError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        self.0.source()
    }
}
