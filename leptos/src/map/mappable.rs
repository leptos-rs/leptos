/// Trait for types that can be mapped over (Result/Option).
pub trait Mappable<T> {
    /// Type of error that maps to the error type of a Result and otherwise is ().
    type Error;

    /// Just like Option::map and Result::map.
    fn map<U>(
        self,
        f: impl Fn(T) -> U,
    ) -> impl Mappable<U, Error = Self::Error>;

    /// Just like Option::unwrap_or_else and Result::unwrap_or_else.
    fn unwrap_or_else(self, f: impl Fn(Self::Error) -> T) -> T;
}

impl<T, E> Mappable<T> for Result<T, E> {
    type Error = E;

    fn map<U>(
        self,
        f: impl Fn(T) -> U,
    ) -> impl Mappable<U, Error = Self::Error> {
        Result::map(self, f)
    }

    fn unwrap_or_else(self, f: impl Fn(E) -> T) -> T {
        match self {
            Ok(v) => v,
            Err(e) => f(e),
        }
    }
}

impl<T> Mappable<T> for Option<T> {
    type Error = ();

    fn map<U>(
        self,
        f: impl Fn(T) -> U,
    ) -> impl Mappable<U, Error = Self::Error> {
        Option::map(self, f)
    }

    fn unwrap_or_else(self, f: impl Fn(()) -> T) -> T {
        match self {
            Some(v) => v,
            None => f(()),
        }
    }
}
