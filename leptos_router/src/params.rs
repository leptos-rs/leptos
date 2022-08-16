use linear_map::LinearMap;

// For now, implemented with a `LinearMap`, as `n` is small enough
// that O(n) iteration over a vectorized map is (*probably*) more space-
// and time-efficient than hashing and using an actual `HashMap`
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Params(pub LinearMap<String, String>);

impl Params {
    pub fn new() -> Self {
        Self(LinearMap::new())
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self(LinearMap::with_capacity(capacity))
    }

    pub fn insert(&mut self, key: String, value: String) -> Option<String> {
        self.0.insert(key, value)
    }
}

impl Default for Params {
    fn default() -> Self {
        Self::new()
    }
}

// Adapted from hash_map! in common_macros crate
// Copyright (c) 2019 Philipp Korber
// https://github.com/rustonaut/common_macros/blob/master/src/lib.rs
#[macro_export]
macro_rules! params {
    ($($key:expr => $val:expr),* ,) => (
        $crate::params!($($key => $val),*)
    );
    ($($key:expr => $val:expr),*) => ({
        let start_capacity = common_macros::const_expr_count!($($key);*);
        #[allow(unused_mut)]
        let mut map = linear_map::LinearMap::with_capacity(start_capacity);
        $( map.insert($key, $val); )*
        $crate::Params(map)
    });
}
