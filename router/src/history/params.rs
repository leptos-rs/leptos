use std::{rc::Rc, str::FromStr};

use linear_map::LinearMap;

use crate::RouterError;

// For now, implemented with a `LinearMap`, as `n` is small enough
// that O(n) iteration over a vectorized map is (*probably*) more space-
// and time-efficient than hashing and using an actual `HashMap`
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ParamsMap(pub LinearMap<String, String>);

impl ParamsMap {
    pub fn new() -> Self {
        Self(LinearMap::new())
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self(LinearMap::with_capacity(capacity))
    }

    pub fn insert(&mut self, key: String, value: String) -> Option<String> {
        self.0.insert(key, value)
    }

    pub fn get(&self, key: &str) -> Option<&String> {
        self.0.get(key)
    }
}

impl Default for ParamsMap {
    fn default() -> Self {
        Self::new()
    }
}

// Adapted from hash_map! in common_macros crate
// Copyright (c) 2019 Philipp Korber
// https://github.com/rustonaut/common_macros/blob/master/src/lib.rs
#[macro_export]
macro_rules! params_map {
    ($($key:expr => $val:expr),* ,) => (
        $crate::ParamsMap!($($key => $val),*)
    );
    ($($key:expr => $val:expr),*) => ({
        let start_capacity = common_macros::const_expr_count!($($key);*);
        #[allow(unused_mut)]
        let mut map = linear_map::LinearMap::with_capacity(start_capacity);
        $( map.insert($key, $val); )*
        $crate::ParamsMap(map)
    });
}

pub trait Params
where
    Self: Sized,
{
    fn from_map(map: &ParamsMap) -> Result<Self, RouterError>;
}

impl Params for () {
    fn from_map(_map: &ParamsMap) -> Result<Self, RouterError> {
        Ok(())
    }
}

pub trait IntoParam
where
    Self: Sized,
{
    fn into_param(value: Option<&str>, name: &str) -> Result<Self, RouterError>;
}

impl<T> IntoParam for Option<T>
where
    T: FromStr,
    <T as FromStr>::Err: std::error::Error + Send + Sync + 'static,
{
    fn into_param(value: Option<&str>, _name: &str) -> Result<Self, RouterError> {
        match value {
            None => Ok(None),
            Some(value) => match T::from_str(value) {
                Ok(value) => Ok(Some(value)),
                Err(e) => {
                    eprintln!("{}", e);
                    Err(RouterError::Params(Rc::new(e)))
                }
            },
        }
    }
}

auto trait NotOption {}
impl<T> !NotOption for Option<T> {}

impl<T> IntoParam for T
where
    T: FromStr + NotOption,
    <T as FromStr>::Err: std::error::Error + Send + Sync + 'static,
{
    fn into_param(value: Option<&str>, name: &str) -> Result<Self, RouterError> {
        let value = value.ok_or_else(|| RouterError::MissingParam(name.to_string()))?;
        Self::from_str(value).map_err(|e| RouterError::Params(Rc::new(e)))
    }
}
