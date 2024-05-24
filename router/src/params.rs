use std::{borrow::Cow, str::FromStr, sync::Arc};
use thiserror::Error;

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct ParamsMap(Vec<(Cow<'static, str>, String)>);

impl ParamsMap {
    /// Creates an empty map.
    #[inline(always)]
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates an empty map with the given capacity.
    #[inline(always)]
    pub fn with_capacity(capacity: usize) -> Self {
        Self(Vec::with_capacity(capacity))
    }

    /*
    /// Inserts a value into the map.
    #[inline(always)]
    pub fn insert(&mut self, key: String, value: String) -> Option<String> {
        use crate::history::url::unescape;
        let value = unescape(&value);
        self.0.insert(key, value)
    }
    */

    /// Gets an owned value from the map.
    #[inline(always)]
    pub fn get(&self, key: &str) -> Option<String> {
        self.0
            .iter()
            .find_map(|(k, v)| (k == key).then_some(v.to_owned()))
    }

    /// Gets a referenc to a value from the map.
    #[inline(always)]
    pub fn get_str(&self, key: &str) -> Option<&str> {
        self.0
            .iter()
            .find_map(|(k, v)| (k == key).then_some(v.as_str()))
    }

    /// Removes a value from the map.
    #[inline(always)]
    pub fn remove(&mut self, key: &str) -> Option<String> {
        for i in 0..self.0.len() {
            if self.0[i].0 == key {
                return Some(self.0.swap_remove(i).1);
            }
        }
        None
    }

    /*
    /// Converts the map to a query string.
    pub fn to_query_string(&self) -> String {
        use crate::history::url::escape;
        let mut buf = String::new();
        if !self.0.is_empty() {
            buf.push('?');
            for (k, v) in &self.0 {
                buf.push_str(&escape(k));
                buf.push('=');
                buf.push_str(&escape(v));
                buf.push('&');
            }
            if buf.len() > 1 {
                buf.pop();
            }
        }
        buf
    }
    */
}

impl<K, V> FromIterator<(K, V)> for ParamsMap
where
    K: Into<Cow<'static, str>>,
    V: Into<String>,
{
    fn from_iter<T: IntoIterator<Item = (K, V)>>(iter: T) -> Self {
        Self(
            iter.into_iter()
                .map(|(k, v)| (k.into(), v.into()))
                .collect(),
        )
    }
}

/// A simple method of deserializing key-value data (like route params or URL search)
/// into a concrete data type. `Self` should typically be a struct in which
/// each field's type implements [`FromStr`].
pub trait Params
where
    Self: Sized,
{
    /// Attempts to deserialize the map into the given type.
    fn from_map(map: &ParamsMap) -> Result<Self, ParamsError>;
}

impl Params for () {
    #[inline(always)]
    fn from_map(_map: &ParamsMap) -> Result<Self, ParamsError> {
        Ok(())
    }
}

pub trait IntoParam
where
    Self: Sized,
{
    fn into_param(value: Option<&str>, name: &str)
        -> Result<Self, ParamsError>;
}

impl<T> IntoParam for Option<T>
where
    T: FromStr,
    <T as FromStr>::Err: std::error::Error + Send + Sync + 'static,
{
    fn into_param(
        value: Option<&str>,
        _name: &str,
    ) -> Result<Self, ParamsError> {
        match value {
            None => Ok(None),
            Some(value) => match T::from_str(value) {
                Ok(value) => Ok(Some(value)),
                Err(e) => Err(ParamsError::Params(Arc::new(e))),
            },
        }
    }
}

// TODO can we support Option<T> and T in a non-nightly way?
#[cfg(feature = "nightly")]
mod option_param {
    auto trait NotOption {}
    impl<T> !NotOption for Option<T> {}

    impl<T> IntoParam for T
    where
        T: FromStr + NotOption,
        <T as FromStr>::Err: std::error::Error + Send + Sync + 'static,
    {
        fn into_param(
            value: Option<&str>,
            name: &str,
        ) -> Result<Self, ParamsError> {
            let value = value
                .ok_or_else(|| ParamsError::MissingParam(name.to_string()))?;
            Self::from_str(value).map_err(|e| ParamsError::Params(Arc::new(e)))
        }
    }
}

/// Errors that can occur while parsing params using [`Params`].
#[derive(Error, Debug, Clone)]
pub enum ParamsError {
    /// A field was missing from the route params.
    #[error("could not find parameter {0}")]
    MissingParam(String),
    /// Something went wrong while deserializing a field.
    #[error("failed to deserialize parameters")]
    Params(Arc<dyn std::error::Error + Send + Sync>),
}

impl PartialEq for ParamsError {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::MissingParam(l0), Self::MissingParam(r0)) => l0 == r0,
            (Self::Params(_), Self::Params(_)) => false,
            _ => false,
        }
    }
}
