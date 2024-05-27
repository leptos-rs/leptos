use linear_map::LinearMap;
use serde::{Deserialize, Serialize};
use std::{str::FromStr, sync::Arc};
use thiserror::Error;

/// A key-value map of the current named route params and their values.
///
/// For now, implemented with a [`LinearMap`], as `n` is small enough
/// that O(n) iteration over a vectorized map is (*probably*) more space-
/// and time-efficient than hashing and using an actual `HashMap`
///
/// [`LinearMap`]: linear_map::LinearMap
#[derive(Debug, PartialEq, Eq, Clone, Deserialize, Serialize)]
#[repr(transparent)]
pub struct ParamsMap(pub LinearMap<String, String>);

impl ParamsMap {
    /// Creates an empty map.
    #[inline(always)]
    pub fn new() -> Self {
        Self(LinearMap::new())
    }

    /// Creates an empty map with the given capacity.
    #[inline(always)]
    pub fn with_capacity(capacity: usize) -> Self {
        Self(LinearMap::with_capacity(capacity))
    }

    /// Inserts a value into the map.
    #[inline(always)]
    pub fn insert(&mut self, key: String, value: String) -> Option<String> {
        self.0.insert(key, value)
    }

    /// Gets a value from the map.
    #[inline(always)]
    pub fn get(&self, key: &str) -> Option<&String> {
        self.0.get(key)
    }

    /// Removes a value from the map.
    #[inline(always)]
    pub fn remove(&mut self, key: &str) -> Option<String> {
        self.0.remove(key)
    }

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
}

impl Default for ParamsMap {
    #[inline(always)]
    fn default() -> Self {
        Self::new()
    }
}

/// Create a [`ParamsMap`] in a declarative style.
///
/// ```
/// # use leptos_router::params_map;
/// # #[cfg(feature = "ssr")] {
/// let map = params_map! {
///     "crate" => "leptos",
///     42 => true, // where key & val: core::fmt::Display
/// };
/// assert_eq!(map.get("crate"), Some(&"leptos".to_string()));
/// assert_eq!(map.get("42"), Some(&true.to_string()))
/// # }
/// ```
// Original implementation included the below credits.
//
// Adapted from hash_map! in common_macros crate
// Copyright (c) 2019 Philipp Korber
// https://github.com/rustonaut/common_macros/blob/master/src/lib.rs
#[macro_export]
macro_rules! params_map {
    // Fast path avoids allocation.
    () => { $crate::ParamsMap::with_capacity(0) };

    // Counting repitions by n = 0 ( + 1 )*
    //
    // https://github.com/rust-lang/rust/issues/83527
    // When stabilized you can use "metavaribale exprs" instead
    //
    // `$key | $val` must be included in the repetition to be valid, it is
    // stringified to null out any possible side-effects.
    ($($key:expr => $val:expr),* $(,)?) => {{
        let n = 0 $(+ { _ = stringify!($key); 1 })*;
        #[allow(unused_mut)]
        let mut map = $crate::ParamsMap::with_capacity(n);
        $( map.insert($key.to_string(), $val.to_string()); )*
        map
    }};
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

cfg_if::cfg_if! {
    if #[cfg(feature = "nightly")] {
        auto trait NotOption {}
        impl<T> !NotOption for Option<T> {}

        impl<T> IntoParam for T
        where
            T: FromStr + NotOption,
            <T as FromStr>::Err: std::error::Error + Send + Sync + 'static,
        {
            fn into_param(value: Option<&str>, name: &str) -> Result<Self, ParamsError> {
                let value = value.ok_or_else(|| ParamsError::MissingParam(name.to_string()))?;
                Self::from_str(value).map_err(|e| ParamsError::Params(Arc::new(e)))
            }
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
