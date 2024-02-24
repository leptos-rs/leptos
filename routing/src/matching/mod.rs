mod horizontal;
mod vertical;
use crate::params::Params;
use alloc::{borrow::Cow, string::String, vec::Vec};
pub use horizontal::*;
pub use vertical::*;

pub struct Routes<Children> {
    base: Cow<'static, str>,
    children: Children,
}

pub struct RouteMatch<'a> {
    matched_nested_routes: Vec<NestedRouteMatch<'a>>,
}

pub struct NestedRouteMatch<'a> {
    /// The portion of the full path matched by this nested route.
    matched_path: String,
    /// The map of params matched by this route.
    params: Params<&'static str>,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct NestedRoute<Segments, Children> {
    segments: Segments,
    children: Children,
}

#[derive(Debug)]
pub struct PartialPathMatch<'a> {
    pub(crate) remaining: &'a str,
    pub(crate) params: Params<&'static str>,
    pub(crate) matched: String,
}

impl<'a> PartialPathMatch<'a> {
    pub fn new(
        remaining: &'a str,
        params: impl Into<Params<&'static str>>,
        matched: impl Into<String>,
    ) -> Self {
        Self {
            remaining,
            params: params.into(),
            matched: matched.into(),
        }
    }

    pub fn is_complete(&self) -> bool {
        self.remaining.is_empty() || self.remaining == "/"
    }

    pub fn remaining(&self) -> &str {
        self.remaining
    }

    pub fn params(&self) -> &[(&'static str, String)] {
        &self.params
    }

    pub fn matched(&self) -> &str {
        self.matched.as_str()
    }
}
