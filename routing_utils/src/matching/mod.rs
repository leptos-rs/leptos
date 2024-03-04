mod horizontal;
mod nested;
mod vertical;
use crate::PathSegment;
use alloc::borrow::Cow;
pub use horizontal::*;
pub use nested::*;
pub use vertical::*;

#[derive(Debug)]
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
    pub fn match_route(&'a self, path: &'a str) -> Option<Children::Match> {
        let path = match &self.base {
            None => path,
            Some(base) => {
                let (base, path) = if base.starts_with('/') {
                    (base.trim_start_matches('/'), path.trim_start_matches('/'))
                } else {
                    (base.as_ref(), path)
                };
                match path.strip_prefix(base) {
                    Some(path) => path,
                    None => return None,
                }
            }
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

pub trait MatchInterface<'a> {
    type Params: IntoIterator<Item = (&'a str, &'a str)>;
    type Child;
    type View;

    fn to_params(&self) -> Self::Params;

    fn to_child(&'a self) -> Self::Child;

    fn to_view(&self) -> Self::View;
}

pub trait MatchNestedRoutes<'a> {
    type Data;
    type Match: MatchInterface<'a>;

    fn match_nested(&'a self, path: &'a str) -> (Option<Self::Match>, &'a str);

    fn generate_routes(
        &self,
    ) -> impl IntoIterator<Item = Vec<PathSegment>> + '_;
}

#[cfg(test)]
mod tests {
    use super::{NestedRoute, ParamSegment, Routes};
    use crate::{
        matching::{MatchInterface, StaticSegment, WildcardSegment},
        PathSegment,
    };

    #[test]
    pub fn matches_single_root_route() {
        let routes = Routes::new(NestedRoute {
            segments: StaticSegment("/"),
            children: (),
            data: (),
            view: || (),
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

        // route generation
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

        let matched = routes.match_route("/author/contact").unwrap();
        assert_eq!(matched.matched(), "");
        assert_eq!(matched.to_child().matched(), "/author/contact");

        let view = matched.to_view();
        assert_eq!(*view, "Home");
        assert_eq!(*matched.to_child().to_view(), "Contact Me");
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
                        view: || (),
                    },
                    NestedRoute {
                        segments: StaticSegment("about"),
                        children: (),
                        data: (),
                        view: || (),
                    },
                ),
                data: (),
                view: || (),
            },
            NestedRoute {
                segments: StaticSegment("/blog"),
                children: (
                    NestedRoute {
                        segments: StaticSegment(""),
                        children: (),
                        data: (),
                        view: || (),
                    },
                    NestedRoute {
                        segments: (StaticSegment("post"), ParamSegment("id")),
                        children: (),
                        data: (),
                        view: || (),
                    },
                ),
                data: (),
                view: || (),
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
                            view: || (),
                        },
                        NestedRoute {
                            segments: StaticSegment("about"),
                            children: (),
                            data: (),
                            view: || (),
                        },
                    ),
                    data: (),
                    view: || (),
                },
                NestedRoute {
                    segments: StaticSegment("/blog"),
                    children: (
                        NestedRoute {
                            segments: StaticSegment(""),
                            children: (),
                            data: (),
                            view: || (),
                        },
                        NestedRoute {
                            segments: StaticSegment("category"),
                            children: (),
                            data: (),
                            view: || (),
                        },
                        NestedRoute {
                            segments: (
                                StaticSegment("post"),
                                ParamSegment("id"),
                            ),
                            children: (),
                            data: (),
                            view: || (),
                        },
                    ),
                    data: (),
                    view: || (),
                },
                NestedRoute {
                    segments: (
                        StaticSegment("/contact"),
                        WildcardSegment("any"),
                    ),
                    children: (),
                    data: (),
                    view: || (),
                },
            ),
            "/portfolio",
        );

        // generates routes correctly
        let (base, _paths) = routes.generate_routes();
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
