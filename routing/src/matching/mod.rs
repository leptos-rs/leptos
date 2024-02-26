mod horizontal;
mod vertical;
use crate::params::Params;
use alloc::{borrow::Cow, string::String, vec::Vec};
pub use horizontal::*;
use std::fmt::Debug;
pub use vertical::*;

pub struct Routes<Children> {
    base: Option<Cow<'static, str>>,
    children: Children,
}

impl<Children> Routes<Children> {
    pub fn new(children: Children) -> Self {
        Self {
            base: None,
            children,
        }
    }

    pub fn new_with_base(
        children: Children,
        base: impl Into<Cow<'static, str>>,
    ) -> Self {
        Self {
            base: Some(base.into()),
            children,
        }
    }
}

impl<Children> Routes<Children>
where
    Children: MatchNestedRoutes,
{
    pub fn match_route<'a>(&self, path: &'a str) -> Option<RouteMatch<'a>> {
        let path = match &self.base {
            None => path,
            Some(base) if base.starts_with('/') => {
                path.trim_start_matches(base.as_ref())
            }
            Some(base) => path
                .trim_start_matches('/')
                .trim_start_matches(base.as_ref()),
        };

        let mut matched_nested_routes = Vec::with_capacity(Children::DEPTH);
        let remaining = self
            .children
            .match_nested_routes(path, &mut matched_nested_routes);

        if matched_nested_routes.is_empty() || !remaining.is_empty() {
            None
        } else {
            Some(RouteMatch {
                path,
                matched_nested_routes,
            })
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct RouteMatch<'a> {
    path: &'a str,
    matched_nested_routes: Vec<NestedRouteMatch<'a>>,
}

impl<'a> RouteMatch<'a> {
    pub fn path(&self) -> &'a str {
        self.path
    }

    pub fn matches(&self) -> &[NestedRouteMatch<'a>] {
        &self.matched_nested_routes
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct NestedRouteMatch<'a> {
    /// The portion of the full path matched only by this nested route.
    matched_path: &'a str,
    /// The map of params matched only by this nested route.
    params: Vec<(&'a str, &'a str)>,
}

impl<'a> NestedRouteMatch<'a> {
    pub fn matched_path(&self) -> &'a str {
        self.matched_path
    }

    pub fn matched_params(&self) -> &[(&'a str, &'a str)] {
        &self.params
    }
}

pub trait MatchNestedRoutes {
    const DEPTH: usize;

    fn matches<'a>(&self, path: &'a str) -> Option<&'a str>;

    fn match_nested_routes<'a>(
        &self,
        path: &'a str,
        matches: &mut Vec<NestedRouteMatch<'a>>,
    ) -> &'a str;
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct NestedRoute<Segments, Children> {
    pub segments: Segments,
    pub children: Children,
}

impl MatchNestedRoutes for () {
    const DEPTH: usize = 0;

    fn matches<'a>(&self, path: &'a str) -> Option<&'a str> {
        Some(path)
    }

    fn match_nested_routes<'a>(
        &self,
        path: &'a str,
        _matches: &mut Vec<NestedRouteMatch<'a>>,
    ) -> &'a str {
        path
    }
}

impl<Segments, Children> MatchNestedRoutes for NestedRoute<Segments, Children>
where
    Self: Debug,
    Segments: PossibleRouteMatch + Debug,
    Children: MatchNestedRoutes,
{
    const DEPTH: usize = Children::DEPTH;

    fn matches<'a>(&self, path: &'a str) -> Option<&'a str> {
        if let Some(remaining) = self.segments.matches(path) {
            self.children.matches(remaining)
        } else {
            None
        }
    }

    fn match_nested_routes<'a>(
        &self,
        mut path: &'a str,
        matches: &mut Vec<NestedRouteMatch<'a>>,
    ) -> &'a str {
        if self.segments.matches(path).is_some() {
            if let Some(PartialPathMatch {
                params,
                matched,
                remaining,
            }) = self.segments.test(path)
            {
                path = remaining;
                matches.push(NestedRouteMatch {
                    matched_path: matched,
                    params,
                });
            }
            return self.children.match_nested_routes(path, matches);
        }

        // otherwise, just return the path as the remainder
        path
    }
}

impl<A> MatchNestedRoutes for (A,)
where
    A: MatchNestedRoutes,
{
    const DEPTH: usize = A::DEPTH;

    fn matches<'a>(&self, path: &'a str) -> Option<&'a str> {
        self.0.matches(path)
    }

    fn match_nested_routes<'a>(
        &self,
        path: &'a str,
        matches: &mut Vec<NestedRouteMatch<'a>>,
    ) -> &'a str {
        self.0.match_nested_routes(path, matches)
    }
}

#[cfg(test)]
mod tests {
    use super::{NestedRoute, Routes};
    use crate::matching::StaticSegment;

    /* #[test]
    pub fn matches_single_root_route() {
        let routes = Routes::new(NestedRoute {
            segments: StaticSegment("/"),
            children: (),
        });
        let matched = routes.match_route("/");
        assert!(matched.is_some());
        let matched = routes.match_route("");
        assert!(matched.is_some())
    }*/

    #[test]
    pub fn matches_nested_route() {
        let routes = Routes::new(NestedRoute {
            segments: StaticSegment(""),
            children: NestedRoute {
                segments: (StaticSegment("author"), StaticSegment("contact")),
                children: (),
            },
        });
        let matched = routes.match_route("/author/contact").unwrap();
        assert_eq!(matched.matches().len(), 2);
        assert_eq!(matched.matches()[0].matched_path, "");
        assert_eq!(matched.matches()[1].matched_path, "/author/contact");
    }

    /*#[test]
    pub fn chooses_between_nested_routes() {
        let routes = Routes::new(NestedRoute {
            segments: StaticSegment("/"),
            children: (
                NestedRoute {
                    segments: StaticSegment(""),
                    children: (),
                },
                NestedRoute {
                    segments: StaticSegment("about"),
                    children: (),
                },
            ),
        });
        let matched = routes.match_route("/");
        panic!("matched = {matched:?}");
    }*/
}

#[derive(Debug)]
pub struct PartialPathMatch<'a> {
    pub(crate) remaining: &'a str,
    pub(crate) params: Vec<(&'a str, &'a str)>,
    pub(crate) matched: &'a str,
}

impl<'a> PartialPathMatch<'a> {
    pub fn new(
        remaining: &'a str,
        params: impl Into<Vec<(&'a str, &'a str)>>,
        matched: &'a str,
    ) -> Self {
        Self {
            remaining,
            params: params.into(),
            matched,
        }
    }

    pub fn is_complete(&self) -> bool {
        self.remaining.is_empty() || self.remaining == "/"
    }

    pub fn remaining(&self) -> &str {
        self.remaining
    }

    pub fn params(&self) -> &[(&'a str, &'a str)] {
        &self.params
    }

    pub fn matched(&self) -> &str {
        self.matched
    }
}

macro_rules! tuples {
    ($($ty:ident),*) => {
        impl<$($ty),*> PossibleRouteMatch for ($($ty,)*)
        where
			$($ty: PossibleRouteMatch),*,
        {
            fn matches_iter(&self, path: &mut Chars) -> bool
            {
                #[allow(non_snake_case)]
                let ($($ty,)*) = &self;
                $($ty.matches_iter(path) &&)* true
            }

            fn test<'a>(&self, path: &'a str) -> Option<PartialPathMatch<'a>>
            {
				let mut full_params = Vec::new();
				let mut full_matched = String::new();
                #[allow(non_snake_case)]
                let ($($ty,)*) = &self;
                $(
                    let PartialPathMatch {
                        remaining,
                        matched,
                        params
                    } = $ty.test(path)?;
                    let path = remaining;
                    full_matched.push_str(&matched);
                    full_params.extend(params);
                )*
                Some(PartialPathMatch {
                    remaining: path,
                    matched: full_matched,
                    params: full_params
                })
            }

            fn generate_path(&self, path: &mut Vec<PathSegment>) {
                #[allow(non_snake_case)]
                let ($($ty,)*) = &self;
                $(
                    $ty.generate_path(path);
                )*
            }
        }
	};
}

//tuples!(A, B);
/*tuples!(A, B, C);
tuples!(A, B, C, D);
tuples!(A, B, C, D, E);
tuples!(A, B, C, D, E, F);
tuples!(A, B, C, D, E, F, G);
tuples!(A, B, C, D, E, F, G, H);
tuples!(A, B, C, D, E, F, G, H, I);
tuples!(A, B, C, D, E, F, G, H, I, J);
tuples!(A, B, C, D, E, F, G, H, I, J, K);
tuples!(A, B, C, D, E, F, G, H, I, J, K, L);
tuples!(A, B, C, D, E, F, G, H, I, J, K, L, M);
tuples!(A, B, C, D, E, F, G, H, I, J, K, L, M, N);
tuples!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O);
tuples!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P);
tuples!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q);
tuples!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R);
tuples!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S);
tuples!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T);
tuples!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U);
tuples!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V);
tuples!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W);
tuples!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X);
tuples!(
    A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y
);
tuples!(
    A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y,
    Z
);*/
