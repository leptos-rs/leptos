use crate::location::{unescape, Url};
use std::{borrow::Cow, str::FromStr, sync::Arc};
use thiserror::Error;

type ParamsMapInner = Vec<(Cow<'static, str>, Vec<String>)>;

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct ParamsMap(ParamsMapInner);

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

    /// Inserts a value into the map.
    pub fn insert(&mut self, key: String, value: String) {
        let value = unescape(&value);

        if let Some(prev) = self.0.iter().position(|(k, _)| k == &key) {
            self.0[prev].1.push(value);
        } else {
            self.0.push((key.into(), vec![value]));
        }
    }

    /// Gets an owned value from the map.
    pub fn get(&self, key: &str) -> Option<Vec<String>> {
        self.0
            .iter()
            .find_map(|(k, v)| (k == key).then_some(v.to_owned()))
    }

    /// Gets a reference to a value from the map.
    pub fn get_str(&self, key: &str) -> Option<&str> {
        self.0.iter().find_map(|(k, v)| {
            if k == key {
                v.last().map(|i| i.as_str())
            } else {
                None
            }
        })
    }

    /// Removes a value from the map.
    #[inline(always)]
    pub fn remove(&mut self, key: &str) -> Option<Vec<String>> {
        for i in 0..self.0.len() {
            if self.0[i].0 == key {
                return Some(self.0.swap_remove(i).1);
            }
        }
        None
    }

    /// Converts the map to a query string.
    pub fn to_query_string(&self) -> String {
        let mut buf = String::new();
        if !self.0.is_empty() {
            buf.push('?');
            for (k, vs) in &self.0 {
                for v in vs {
                    buf.push_str(&Url::escape(k));
                    buf.push('=');
                    buf.push_str(&Url::escape(v));
                    buf.push('&');
                }
            }
            if buf.len() > 1 {
                buf.pop();
            }
        }
        buf
    }
}

impl<K, V> FromIterator<(K, V)> for ParamsMap
where
    K: Into<Cow<'static, str>>,
    V: Into<String>,
{
    fn from_iter<T: IntoIterator<Item = (K, V)>>(iter: T) -> Self {
        Self(
            iter.into_iter()
                .map(|(k, v)| (k.into(), vec![v.into()]))
                .collect(),
        )
    }
}

impl IntoIterator for ParamsMap {
    type Item = (Cow<'static, str>, String);
    type IntoIter = ParamsMapIter;

    fn into_iter(self) -> Self::IntoIter {
        let inner = self.0.into_iter().fold(vec![], |mut c, (k, vs)| {
            for v in vs {
                c.push((k.clone(), v));
            }
            c
        });
        ParamsMapIter(inner.into_iter())
    }
}

/// An iterator over the keys and values of a [`ParamsMap`].
#[derive(Debug)]
pub struct ParamsMapIter(
    <Vec<(Cow<'static, str>, String)> as IntoIterator>::IntoIter,
);

impl Iterator for ParamsMapIter {
    type Item = (Cow<'static, str>, String);

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
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
    use super::{IntoParam, ParamsError};
    use std::{str::FromStr, sync::Arc};

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paramsmap_to_query_string() {
        let mut map = ParamsMap::new();
        let key = "param".to_string();
        let value1 = "a".to_string();
        let value2 = "b".to_string();
        map.insert(key.clone(), value1);
        map.insert(key, value2);
        let query_string = map.to_query_string();
        assert_eq!(&query_string, "?param=a&param=b")
    }
}
