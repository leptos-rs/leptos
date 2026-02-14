use crate::location::Url;
use std::{borrow::Cow, str::FromStr, sync::Arc};
use thiserror::Error;

type ParamsMapInner = Vec<(Cow<'static, str>, Vec<String>)>;

/// A key-value map of the current named route params and their values.
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
    ///
    /// If a value with that key already exists, the new value will be added to it.
    /// To replace the value instead, see [`replace`](Self::replace).
    pub fn insert(&mut self, key: impl Into<Cow<'static, str>>, value: String) {
        let value = Url::unescape(&value);

        let key = key.into();
        if let Some(prev) = self.0.iter_mut().find(|(k, _)| k == &key) {
            prev.1.push(value);
        } else {
            self.0.push((key, vec![value]));
        }
    }

    /// Inserts a value into the map, replacing any existing value for that key.
    pub fn replace(
        &mut self,
        key: impl Into<Cow<'static, str>>,
        value: String,
    ) {
        let value = Url::unescape(&value);

        let key = key.into();
        if let Some(prev) = self.0.iter_mut().find(|(k, _)| k == &key) {
            prev.1.clear();
            prev.1.push(value);
        } else {
            self.0.push((key, vec![value]));
        }
    }

    /// Gets the most-recently-added value of this param from the map.
    pub fn get(&self, key: &str) -> Option<String> {
        self.get_str(key).map(ToOwned::to_owned)
    }

    /// Gets all references to a param of this name from the map.
    pub fn get_all(&self, key: &str) -> Option<Vec<String>> {
        self.0
            .iter()
            .find_map(|(k, v)| if k == key { Some(v.clone()) } else { None })
    }

    /// Gets an iterator for all the most-recently-added values on the map
    pub fn latest_values(&self) -> ParamsMapIterRef<'_> {
        let inner: Vec<_> = self
            .0
            .iter()
            .flat_map(|(k, v)| v.last().map(|v| (k, v.as_str())))
            .collect();
        ParamsMapIterRef(inner.into_iter())
    }

    /// Gets a reference to the most-recently-added value of this param from the map.
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
        let mut map = Self::new();

        for (key, value) in iter {
            map.insert(key, value.into());
        }
        map
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

impl<'a> IntoIterator for &'a ParamsMap {
    type Item = (&'a Cow<'static, str>, &'a str);
    type IntoIter = ParamsMapIterRef<'a>;

    fn into_iter(self) -> Self::IntoIter {
        let inner: Vec<_> = self
            .0
            .iter()
            .flat_map(|(k, v)| v.iter().map(move |v| (k, v.as_str())))
            .collect();
        ParamsMapIterRef(inner.into_iter())
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

/// An iterator over the references of the keys and values of a [`ParamMap`].
#[derive(Debug)]
pub struct ParamsMapIterRef<'a>(
    <Vec<(&'a Cow<'static, str>, &'a str)> as IntoIterator>::IntoIter,
);

impl<'a> Iterator for ParamsMapIterRef<'a> {
    type Item = (&'a Cow<'static, str>, &'a str);

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

/// Converts some parameter value from the URL into a typed parameter with the given name.
pub trait IntoParam
where
    Self: Sized,
{
    /// Converts the param.
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

/// Helpers for the `Params` derive macro to allow specialization without nightly.
pub mod macro_helpers {
    use crate::params::{IntoParam, ParamsError};
    use std::{str::FromStr, sync::Arc};

    /// This struct is never actually created; it just exists so that we can impl associated
    /// functions on it.
    pub struct Wrapper<T>(T);

    impl<T: IntoParam> Wrapper<T> {
        /// This is the 'preferred' impl to be used for all `T` that implement `IntoParam`.
        /// Because it is directly on the struct, the compiler will pick this over the impl from
        /// the `Fallback` trait.
        #[inline]
        pub fn __into_param(
            value: Option<&str>,
            name: &str,
        ) -> Result<T, ParamsError> {
            T::into_param(value, name)
        }
    }

    /// If the Fallback trait is in scope, then the compiler has two possible implementations for
    /// `__into_params`. It will pick the one from this trait if the inherent one doesn't exist.
    /// (which it won't if `T` does not implement `IntoParam`)
    pub trait Fallback<T>: Sized
    where
        T: FromStr,
        <T as FromStr>::Err: std::error::Error + Send + Sync + 'static,
    {
        /// Fallback function in case the inherent impl on the Wrapper struct does not exist for
        /// `T`
        #[inline]
        fn __into_param(
            value: Option<&str>,
            name: &str,
        ) -> Result<T, ParamsError> {
            let value = value
                .ok_or_else(|| ParamsError::MissingParam(name.to_string()))?;
            T::from_str(value).map_err(|e| ParamsError::Params(Arc::new(e)))
        }
    }

    impl<T> Fallback<T> for Wrapper<T>
    where
        T: FromStr,
        <T as FromStr>::Err: std::error::Error + Send + Sync + 'static,
    {
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

#[cfg(all(test, feature = "ssr"))]
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
