use linear_map::LinearMap;
use std::{
    any::{Any, TypeId},
    collections::HashMap,
    future::Future,
    hash::{BuildHasherDefault, Hash, Hasher},
    pin::Pin,
    rc::Rc,
};

/// Optimized hasher for `TypeId`
/// See https://github.com/chris-morgan/anymap/blob/2e9a570491664eea18ad61d98aa1c557d5e23e67/src/lib.rs#L599
/// and https://github.com/actix/actix-web/blob/97399e8c8ce584d005577604c10bd391e5da7268/actix-http/src/extensions.rs#L8
#[derive(Debug, Default)]
#[doc(hidden)]
struct TypeIdHasher(u64);

impl Hasher for TypeIdHasher {
    fn write(&mut self, bytes: &[u8]) {
        unimplemented!(
            "This TypeIdHasher can only handle u64s, not {:?}",
            bytes
        );
    }

    fn write_u64(&mut self, i: u64) {
        self.0 = i;
    }

    fn finish(&self) -> u64 {
        self.0
    }
}

/// A context that can be used to store application data that should be available when resolving static routes.
/// This is useful for things like database connections to pull dynamic path parameters from.
///
/// Note that this context will be reused for every route, so you should not store any
/// route-specific data in it, nor mutate any data in it.
#[derive(Debug, Default)]
pub struct StaticRenderContext(
    HashMap<TypeId, Box<dyn Any>, BuildHasherDefault<TypeIdHasher>>,
);

impl StaticRenderContext {
    #[doc(hidden)]
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert a value into the context.
    ///
    /// # Example
    /// ```rust
    /// use leptos::StaticRenderContext;
    ///
    /// let mut context = StaticRenderContext::new();
    /// context.insert(42);
    /// ```
    #[inline]
    pub fn insert<T: 'static>(&mut self, value: T) {
        self.0.insert(TypeId::of::<T>(), Box::new(value));
    }

    /// Get a value from the context.
    ///
    /// # Example
    /// ```rust
    /// use leptos::StaticRenderContext;
    ///
    /// let mut context = StaticRenderContext::new();
    /// context.insert(42);
    /// assert_eq!(context.get::<i32>(), Some(&42));
    /// ```
    #[inline]
    pub fn get<T: 'static>(&self) -> Option<&T> {
        self.0
            .get(&TypeId::of::<T>())
            .and_then(|v| v.downcast_ref())
    }
}

#[derive(Debug, Default)]
pub struct StaticParamsMap(pub LinearMap<String, Vec<String>>);

impl StaticParamsMap {
    /// Create a new empty `StaticParamsMap`.
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert a value into the map.
    #[inline]
    pub fn insert(&mut self, key: impl ToString, value: Vec<String>) {
        self.0.insert(key.to_string(), value);
    }

    /// Get a value from the map.
    #[inline]
    pub fn get(&self, key: &str) -> Option<&Vec<String>> {
        self.0.get(key)
    }
}

#[doc(hidden)]
#[derive(Debug)]
pub struct StaticPath<'b, 'a: 'b> {
    path: &'a str,
    segments: Vec<StaticPathSegment<'a>>,
    params: LinearMap<&'a str, &'b Vec<String>>,
}

#[doc(hidden)]
#[derive(Debug)]
enum StaticPathSegment<'a> {
    Static(&'a str),
    Param(&'a str),
    Wildcard(&'a str),
}

impl<'b, 'a: 'b> StaticPath<'b, 'a> {
    pub fn new(path: &'a str) -> StaticPath<'b, 'a> {
        use StaticPathSegment::*;
        Self {
            path,
            segments: path
                .split("/")
                .filter(|s| s.len() > 0)
                .map(|s| match s.chars().next() {
                    Some(':') => Param(&s[1..]),
                    Some('*') => Wildcard(&s[1..]),
                    _ => Static(s),
                })
                .collect::<Vec<_>>(),
            params: LinearMap::new(),
        }
    }

    pub fn add_params(&mut self, params: &'b StaticParamsMap) {
        use StaticPathSegment::*;
        for segment in self.segments.iter() {
            match segment {
                Param(name) | Wildcard(name) => {
                    if let Some(value) = params.get(name) {
                        self.params.insert(name, value);
                    }
                }
                _ => {}
            }
        }
    }

    pub fn into_paths(self) -> Vec<String> {
        use StaticPathSegment::*;
        let mut paths = vec![String::new()];

        let empty = vec!["".to_string()];

        for segment in self.segments {
            match segment {
                Static(s) => {
                    paths = paths
                        .into_iter()
                        .map(|p| format!("{}/{}", p, s))
                        .collect::<Vec<_>>();
                }
                Param(name) | Wildcard(name) => {
                    let mut new_paths = vec![];
                    for path in paths {
                        for val in
                            self.params.get(name).unwrap_or(&&empty).iter()
                        {
                            new_paths.push(format!("{}/{}", path, val));
                        }
                    }
                    paths = new_paths;
                }
            }
        }
        paths
    }

    pub fn parent(&self) -> Option<StaticPath<'b, 'a>> {
        if self.path == "/" || self.path == "" {
            return None;
        }
        self.path
            .rfind('/')
            .map(|i| StaticPath::new(&self.path[..i]))
    }

    pub fn parents(&self) -> Vec<StaticPath<'b, 'a>> {
        let mut parents = vec![];
        let mut parent = self.parent();
        while let Some(p) = parent {
            parent = p.parent();
            parents.push(p);
        }
        parents
    }

    pub fn path(&self) -> &'a str {
        self.path
    }
}

impl Hash for StaticPath<'_, '_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.path.hash(state);
    }
}

pub type StaticData = Rc<StaticDataFn>;

pub type StaticDataFn = dyn Fn(&StaticRenderContext) -> Pin<Box<dyn Future<Output = StaticParamsMap>>>
    + 'static;
