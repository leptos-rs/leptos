mod horizontal;
mod vertical;
use alloc::{borrow::Cow, vec::Vec};
use core::iter;
pub use horizontal::*;
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

impl<'a, Children> Routes<Children>
where
    Children: MatchNestedRoutes<'a>,
{
    pub fn match_route(&self, path: &'a str) -> Option<Children::Match> {
        let path = match &self.base {
            None => path,
            Some(base) if base.starts_with('/') => {
                path.trim_start_matches(base.as_ref())
            }
            Some(base) => path
                .trim_start_matches('/')
                .trim_start_matches(base.as_ref()),
        };

        let (matched, remaining) = self.children.match_nested(path);
        let matched = matched?;

        if !remaining.is_empty() {
            None
        } else {
            Some(matched)
        }
    }
}

pub trait IntoParams<'a> {
    type IntoParams: IntoIterator<Item = (&'a str, &'a str)>;

    fn to_params(&self) -> Self::IntoParams;
}

pub trait MatchNestedRoutes<'a> {
    type Data;
    type ParamsIter: IntoIterator<Item = (&'a str, &'a str)> + Clone;
    type Match: IntoParams<'a>;

    const DEPTH: usize;

    fn matches(&self, path: &'a str) -> Option<&'a str>;

    fn match_nested(&self, path: &'a str) -> (Option<Self::Match>, &'a str);
}

#[derive(Debug, PartialEq, Eq)]
pub struct NestedMatch<'a, ParamsIter, Child> {
    /// The portion of the full path matched only by this nested route.
    matched: &'a str,
    /// The map of params matched only by this nested route.
    params: ParamsIter,
    /// The nested route.
    child: Child,
}

impl<'a, ParamsIter, Child> IntoParams<'a>
    for NestedMatch<'a, ParamsIter, Child>
where
    ParamsIter: IntoIterator<Item = (&'a str, &'a str)> + Clone,
{
    type IntoParams = ParamsIter;

    fn to_params(&self) -> Self::IntoParams {
        self.params.clone()
    }
}

impl<'a, ParamsIter, Child> NestedMatch<'a, ParamsIter, Child> {
    pub fn matched(&self) -> &'a str {
        self.matched
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct NestedRoute<Segments, Children, Data, View> {
    pub segments: Segments,
    pub children: Children,
    pub data: Data,
    pub view: View,
}

impl<'a> MatchNestedRoutes<'a> for () {
    type Data = ();
    type ParamsIter = iter::Empty<(&'a str, &'a str)>;
    type Match = ();

    const DEPTH: usize = 0;

    fn matches(&self, path: &'a str) -> Option<&'a str> {
        Some(path)
    }

    fn match_nested(&self, path: &'a str) -> (Option<Self::Match>, &'a str) {
        (Some(()), path)
    }
}

impl<'a> IntoParams<'a> for () {
    type IntoParams = iter::Empty<(&'a str, &'a str)>;

    fn to_params(&self) -> Self::IntoParams {
        iter::empty()
    }
}

impl<'a, Segments, Children, Data, View> MatchNestedRoutes<'a>
    for NestedRoute<Segments, Children, Data, View>
where
    Segments: PossibleRouteMatch,
    Children: MatchNestedRoutes<'a>,
    <Segments::ParamsIter<'a> as IntoIterator>::IntoIter: Clone,
    <<Children::Match as IntoParams<'a>>::IntoParams as IntoIterator>::IntoIter:
        Clone,
{
    type Data = Data;
    type ParamsIter = iter::Chain<
        <Segments::ParamsIter<'a> as IntoIterator>::IntoIter,
        <<Children::Match as IntoParams<'a>>::IntoParams as IntoIterator>::IntoIter,
    >;
    type Match = NestedMatch<'a, Self::ParamsIter, Children::Match>;

    const DEPTH: usize = Children::DEPTH;

    fn matches(&self, path: &'a str) -> Option<&'a str> {
        if let Some(remaining) = self.segments.matches(path) {
            self.children.matches(remaining)
        } else {
            None
        }
    }

    fn match_nested(&self, path: &'a str) -> (Option<Self::Match>, &'a str) {
        self.segments
            .test(path)
            .and_then(
                |PartialPathMatch {
                     remaining,
                     params,
                     matched,
                 }| {
                    let (inner, remaining) =
                        self.children.match_nested(remaining);
                    let inner = inner?;
                    let params = params.into_iter();

                    Some((
                        Some(NestedMatch {
                            matched,
                            params: params.chain(inner.to_params()),
                            child: inner,
                        }),
                        remaining,
                    ))
                },
            )
            .unwrap_or((None, path))
    }
}

impl<'a, A> MatchNestedRoutes<'a> for (A,)
where
    A: MatchNestedRoutes<'a>,
{
    type Data = A::Data;
    type ParamsIter = A::ParamsIter;
    type Match = A::Match;

    const DEPTH: usize = A::DEPTH;

    fn matches(&self, path: &'a str) -> Option<&'a str> {
        self.0.matches(path)
    }

    fn match_nested(&self, path: &'a str) -> (Option<Self::Match>, &'a str) {
        self.0.match_nested(path)
    }
}

#[cfg(test)]
mod tests {
    use super::{NestedRoute, Routes};
    use crate::matching::StaticSegment;

    #[test]
    pub fn matches_single_root_route() {
        let routes = Routes::new(NestedRoute {
            segments: StaticSegment("/"),
            children: (),
            data: (),
            view: (),
        });
        let matched = routes.match_route("/");
        assert!(matched.is_some());
        let matched = routes.match_route("");
        assert!(matched.is_some())
    }

    #[test]
    pub fn matches_nested_route() {
        let routes = Routes::new(NestedRoute {
            segments: StaticSegment(""),
            children: NestedRoute {
                segments: (StaticSegment("author"), StaticSegment("contact")),
                children: (),
                data: (),
                view: "Contact Me",
            },
            data: (),
            view: "Home",
        });
        let matched = routes.match_route("/author/contact").unwrap();
        assert_eq!(matched.matched, "");
        assert_eq!(matched.child.matched, "/author/contact");
    }

    #[test]
    pub fn does_not_match_incomplete_route() {
        let routes = Routes::new(NestedRoute {
            segments: StaticSegment(""),
            children: NestedRoute {
                segments: (StaticSegment("author"), StaticSegment("contact")),
                children: (),
                data: (),
                view: "Contact Me",
            },
            data: (),
            view: "Home",
        });
        let matched = routes.match_route("/");
        assert!(matched.is_none());
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
pub struct PartialPathMatch<'a, ParamsIter> {
    pub(crate) remaining: &'a str,
    pub(crate) params: ParamsIter,
    pub(crate) matched: &'a str,
}

impl<'a, ParamsIter> PartialPathMatch<'a, ParamsIter> {
    pub fn new(
        remaining: &'a str,
        params: ParamsIter,
        matched: &'a str,
    ) -> Self {
        Self {
            remaining,
            params,
            matched,
        }
    }

    pub fn is_complete(&self) -> bool {
        self.remaining.is_empty() || self.remaining == "/"
    }

    pub fn remaining(&self) -> &str {
        self.remaining
    }

    pub fn params(self) -> ParamsIter {
        self.params
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
