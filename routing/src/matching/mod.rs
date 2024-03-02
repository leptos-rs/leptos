mod horizontal;
mod vertical;
use crate::PathSegment;
use alloc::borrow::Cow;
use core::iter;
use either_of::*;
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
            Some(base) => {
                let (base, path) = if base.starts_with('/') {
                    (base.trim_start_matches('/'), path.trim_start_matches('/'))
                } else {
                    (base.as_ref(), path)
                };
                if let Some(path) = path.strip_prefix(base) {
                    path
                } else {
                    return None;
                }
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

    pub fn generate_routes(
        &'a self,
    ) -> (
        Option<&str>,
        impl IntoIterator<Item = Vec<PathSegment>> + 'a,
    ) {
        (self.base.as_deref(), self.children.generate_routes())
    }
}

pub trait IntoParams<'a> {
    type IntoParams: IntoIterator<Item = (&'a str, &'a str)>;

    fn to_params(&self) -> Self::IntoParams;
}

pub trait MatchNestedRoutes<'a> {
    type Data;
    //type ParamsIter: IntoIterator<Item = (&'a str, &'a str)> + Clone;
    type Match: IntoParams<'a>;

    fn match_nested(&self, path: &'a str) -> (Option<Self::Match>, &'a str);

    fn generate_routes(
        &self,
    ) -> impl IntoIterator<Item = Vec<PathSegment>> + '_;
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
    //type ParamsIter = iter::Empty<(&'a str, &'a str)>;
    type Match = ();

    fn match_nested(&self, path: &'a str) -> (Option<Self::Match>, &'a str) {
        (Some(()), path)
    }

    fn generate_routes(
        &self,
    ) -> impl IntoIterator<Item = Vec<PathSegment>> + '_ {
        iter::once(vec![PathSegment::Unit])
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
    type Match = NestedMatch<'a, iter::Chain<
        <Segments::ParamsIter<'a> as IntoIterator>::IntoIter,
        <<Children::Match as IntoParams<'a>>::IntoParams as IntoIterator>::IntoIter,
    >, Children::Match>;

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

                    if remaining.is_empty() {
                        Some((
                            Some(NestedMatch {
                                matched,
                                params: params.chain(inner.to_params()),
                                child: inner,
                            }),
                            remaining,
                        ))
                    } else {
                        None
                    }
                },
            )
            .unwrap_or((None, path))
    }

    fn generate_routes(
        &self,
    ) -> impl IntoIterator<Item = Vec<PathSegment>> + '_ {
        let mut segment_routes = Vec::new();
        self.segments.generate_path(&mut segment_routes);
        let segment_routes = segment_routes.into_iter();
        let children_routes = self.children.generate_routes().into_iter();
        children_routes.map(move |child_routes| {
            segment_routes
                .clone()
                .chain(child_routes)
                .filter(|seg| seg != &PathSegment::Unit)
                .collect()
        })
    }
}

impl<'a, A> MatchNestedRoutes<'a> for (A,)
where
    A: MatchNestedRoutes<'a>,
{
    type Data = A::Data;
    type Match = A::Match;

    fn match_nested(&self, path: &'a str) -> (Option<Self::Match>, &'a str) {
        self.0.match_nested(path)
    }

    fn generate_routes(
        &self,
    ) -> impl IntoIterator<Item = Vec<PathSegment>> + '_ {
        self.0.generate_routes()
    }
}

#[cfg(test)]
mod tests {
    use super::{NestedRoute, ParamSegment, Routes};
    use crate::{
        matching::{IntoParams, StaticSegment, WildcardSegment},
        PathSegment,
    };

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
        assert!(matched.is_some());
        let (base, paths) = routes.generate_routes();
        assert_eq!(base, None);
        let paths = paths.into_iter().collect::<Vec<_>>();
        assert_eq!(paths, vec![vec![PathSegment::Static("/".into())]]);
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
        let (base, paths) = routes.generate_routes();
        assert_eq!(base, None);
        let paths = paths.into_iter().collect::<Vec<_>>();
        assert_eq!(
            paths,
            vec![vec![
                PathSegment::Static("".into()),
                PathSegment::Static("author".into()),
                PathSegment::Static("contact".into())
            ]]
        );
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

    #[test]
    pub fn chooses_between_nested_routes() {
        let routes = Routes::new((
            NestedRoute {
                segments: StaticSegment("/"),
                children: (
                    NestedRoute {
                        segments: StaticSegment(""),
                        children: (),
                        data: (),
                        view: (),
                    },
                    NestedRoute {
                        segments: StaticSegment("about"),
                        children: (),
                        data: (),
                        view: (),
                    },
                ),
                data: (),
                view: (),
            },
            NestedRoute {
                segments: StaticSegment("/blog"),
                children: (
                    NestedRoute {
                        segments: StaticSegment(""),
                        children: (),
                        data: (),
                        view: (),
                    },
                    NestedRoute {
                        segments: (StaticSegment("post"), ParamSegment("id")),
                        children: (),
                        data: (),
                        view: (),
                    },
                ),
                data: (),
                view: (),
            },
        ));

        // generates routes correctly
        let (base, paths) = routes.generate_routes();
        assert_eq!(base, None);
        let paths = paths.into_iter().collect::<Vec<_>>();
        assert_eq!(
            paths,
            vec![
                vec![
                    PathSegment::Static("/".into()),
                    PathSegment::Static("".into()),
                ],
                vec![
                    PathSegment::Static("/".into()),
                    PathSegment::Static("about".into())
                ],
                vec![
                    PathSegment::Static("/blog".into()),
                    PathSegment::Static("".into()),
                ],
                vec![
                    PathSegment::Static("/blog".into()),
                    PathSegment::Static("post".into()),
                    PathSegment::Param("id".into())
                ]
            ]
        );

        let matched = routes.match_route("/about").unwrap();
        let params = matched.to_params().collect::<Vec<_>>();
        assert!(params.is_empty());
        let matched = routes.match_route("/blog").unwrap();
        let params = matched.to_params().collect::<Vec<_>>();
        assert!(params.is_empty());
        let matched = routes.match_route("/blog/post/42").unwrap();
        let params = matched.to_params().collect::<Vec<_>>();
        assert_eq!(params, vec![("id", "42")]);
    }

    #[test]
    pub fn arbitrary_nested_routes() {
        let routes = Routes::new_with_base(
            (
                NestedRoute {
                    segments: StaticSegment("/"),
                    children: (
                        NestedRoute {
                            segments: StaticSegment("/"),
                            children: (),
                            data: (),
                            view: (),
                        },
                        NestedRoute {
                            segments: StaticSegment("about"),
                            children: (),
                            data: (),
                            view: (),
                        },
                    ),
                    data: (),
                    view: (),
                },
                NestedRoute {
                    segments: StaticSegment("/blog"),
                    children: (
                        NestedRoute {
                            segments: StaticSegment(""),
                            children: (),
                            data: (),
                            view: (),
                        },
                        NestedRoute {
                            segments: StaticSegment("category"),
                            children: (),
                            data: (),
                            view: (),
                        },
                        NestedRoute {
                            segments: (
                                StaticSegment("post"),
                                ParamSegment("id"),
                            ),
                            children: (),
                            data: (),
                            view: (),
                        },
                    ),
                    data: (),
                    view: (),
                },
                NestedRoute {
                    segments: (
                        StaticSegment("/contact"),
                        WildcardSegment("any"),
                    ),
                    children: (),
                    data: (),
                    view: (),
                },
            ),
            "/portfolio",
        );

        // generates routes correctly
        let (base, paths) = routes.generate_routes();
        assert_eq!(base, Some("/portfolio"));

        let matched = routes.match_route("/about");
        assert!(matched.is_none());

        let matched = routes.match_route("/portfolio/about").unwrap();
        let params = matched.to_params().collect::<Vec<_>>();
        assert!(params.is_empty());

        let matched = routes.match_route("/portfolio/blog/post/42").unwrap();
        let params = matched.to_params().collect::<Vec<_>>();
        assert_eq!(params, vec![("id", "42")]);

        let matched = routes.match_route("/portfolio/contact").unwrap();
        let params = matched.to_params().collect::<Vec<_>>();
        assert_eq!(params, vec![("any", "")]);

        let matched = routes.match_route("/portfolio/contact/foobar").unwrap();
        let params = matched.to_params().collect::<Vec<_>>();
        assert_eq!(params, vec![("any", "foobar")]);
    }
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

macro_rules! chain_generated {
    ($first:expr, $second:expr, ) => {
        $first.chain($second)
    };
    ($first:expr, $second:ident, $($rest:ident,)+) => {
        chain_generated!(
            $first.chain($second),
            $($rest,)+
        )
    }
}

impl<'a, A, B> IntoParams<'a> for Either<A, B>
where
    A: IntoParams<'a>,
    B: IntoParams<'a>,
{
    type IntoParams = Either<
        <A::IntoParams as IntoIterator>::IntoIter,
        <B::IntoParams as IntoIterator>::IntoIter,
    >;

    fn to_params(&self) -> Self::IntoParams {
        match self {
            Either::Left(i) => Either::Left(i.to_params().into_iter()),
            Either::Right(i) => Either::Right(i.to_params().into_iter()),
        }
    }
}

impl<'a, A, B> MatchNestedRoutes<'a> for (A, B)
where
    A: MatchNestedRoutes<'a>,
    B: MatchNestedRoutes<'a>,
{
    type Data = (A::Data, B::Data);
    type Match = Either<A::Match, B::Match>;

    fn match_nested(&self, path: &'a str) -> (Option<Self::Match>, &'a str) {
        #[allow(non_snake_case)]
        let (A, B) = &self;
        if let (Some(matched), remaining) = A.match_nested(path) {
            return (Some(Either::Left(matched)), remaining);
        }
        if let (Some(matched), remaining) = B.match_nested(path) {
            return (Some(Either::Right(matched)), remaining);
        }
        (None, path)
    }

    fn generate_routes(
        &self,
    ) -> impl IntoIterator<Item = Vec<PathSegment>> + '_ {
        #[allow(non_snake_case)]
        let (A, B) = &self;
        #[allow(non_snake_case)]
        let A = A.generate_routes().into_iter();
        #[allow(non_snake_case)]
        let B = B.generate_routes().into_iter();
        A.chain(B)
    }
}

macro_rules! tuples {
    ($either:ident => $($ty:ident),*) => {
        impl<'a, $($ty,)*> IntoParams<'a> for $either <$($ty,)*>
        where
			$($ty: IntoParams<'a>),*,
        {
            type IntoParams = $either<$(
                <$ty::IntoParams as IntoIterator>::IntoIter,
            )*>;

            fn to_params(&self) -> Self::IntoParams {
                match self {
                    $($either::$ty(i) => $either::$ty(i.to_params().into_iter()),)*
                }
            }
        }

        impl<'a, $($ty),*> MatchNestedRoutes<'a> for ($($ty,)*)
        where
			$($ty: MatchNestedRoutes<'a>),*,
        {
            type Data = ($($ty::Data,)*);
            type Match = $either<$($ty::Match,)*>;

            fn match_nested(&self, path: &'a str) -> (Option<Self::Match>, &'a str) {
                #[allow(non_snake_case)]
                let ($($ty,)*) = &self;
                $(if let (Some(matched), remaining) = $ty.match_nested(path) {
                    return (Some($either::$ty(matched)), remaining);
                })*
                (None, path)
            }

            fn generate_routes(
                &self,
            ) -> impl IntoIterator<Item = Vec<PathSegment>> + '_ {
                #[allow(non_snake_case)]
                let ($($ty,)*) = &self;
                #[allow(non_snake_case)]
                $(let $ty = $ty.generate_routes().into_iter();)*
                chain_generated!($($ty,)*)
            }
        }
    }
}

tuples!(EitherOf3 => A, B, C);
tuples!(EitherOf4 => A, B, C, D);
tuples!(EitherOf5 => A, B, C, D, E);
tuples!(EitherOf6 => A, B, C, D, E, F);
tuples!(EitherOf7 => A, B, C, D, E, F, G);
tuples!(EitherOf8 => A, B, C, D, E, F, G, H);
tuples!(EitherOf9 => A, B, C, D, E, F, G, H, I);
tuples!(EitherOf10 => A, B, C, D, E, F, G, H, I, J);
tuples!(EitherOf11 => A, B, C, D, E, F, G, H, I, J, K);
tuples!(EitherOf12 => A, B, C, D, E, F, G, H, I, J, K, L);
tuples!(EitherOf13 => A, B, C, D, E, F, G, H, I, J, K, L, M);
tuples!(EitherOf14 => A, B, C, D, E, F, G, H, I, J, K, L, M, N);
tuples!(EitherOf15 => A, B, C, D, E, F, G, H, I, J, K, L, M, N, O);
tuples!(EitherOf16 => A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P);
