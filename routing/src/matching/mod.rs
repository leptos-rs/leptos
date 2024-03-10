mod choose_view;
mod path_segment;
pub use choose_view::*;
pub use path_segment::*;
mod horizontal;
mod nested;
mod vertical;
pub use horizontal::*;
pub use nested::*;
use std::{borrow::Cow, marker::PhantomData};
use tachys::{
    renderer::Renderer,
    view::{any_view::IntoAny, Render, RenderHtml},
};
pub use vertical::*;

#[derive(Debug)]
pub struct Routes<Children, Rndr> {
    base: Option<Cow<'static, str>>,
    children: Children,
    ty: PhantomData<Rndr>,
}

impl<Children, Rndr> Routes<Children, Rndr> {
    pub fn new(children: Children) -> Self {
        Self {
            base: None,
            children,
            ty: PhantomData,
        }
    }

    pub fn new_with_base(
        children: Children,
        base: impl Into<Cow<'static, str>>,
    ) -> Self {
        Self {
            base: Some(base.into()),
            children,
            ty: PhantomData,
        }
    }
}

impl<Children, Rndr> Routes<Children, Rndr>
where
    Rndr: Renderer + 'static,
    Children: MatchNestedRoutes<Rndr>,
{
    pub fn match_route(&self, path: &str) -> Option<Children::Match> {
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

        if !(remaining.is_empty() || remaining == "/") {
            None
        } else {
            Some(matched.1)
        }
    }

    pub fn generate_routes(
        &self,
    ) -> (
        Option<&str>,
        impl IntoIterator<Item = Vec<PathSegment>> + '_,
    ) {
        (self.base.as_deref(), self.children.generate_routes())
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct RouteMatchId(pub(crate) u16);

pub trait MatchInterface<R>
where
    R: Renderer + 'static,
{
    type Child: MatchInterface<R> + MatchParams + 'static;
    type View: Render<R> + RenderHtml<R> + 'static;

    fn as_id(&self) -> RouteMatchId;

    fn as_matched(&self) -> &str;

    fn into_view_and_child(
        self,
    ) -> (impl ChooseView<R, Output = Self::View>, Option<Self::Child>);
}

pub trait MatchParams {
    type Params: IntoIterator<Item = (Cow<'static, str>, String)>;

    fn to_params(&self) -> Self::Params;
}

pub trait MatchNestedRoutes<R>
where
    R: Renderer + 'static,
{
    type Data;
    type View;
    type Match: MatchInterface<R> + MatchParams;

    fn match_nested<'a>(
        &'a self,
        path: &'a str,
    ) -> (Option<(RouteMatchId, Self::Match)>, &str);

    fn generate_routes(
        &self,
    ) -> impl IntoIterator<Item = Vec<PathSegment>> + '_;
}

#[cfg(test)]
mod tests {
    use super::{NestedRoute, ParamSegment, Routes};
    use crate::{MatchInterface, PathSegment, StaticSegment, WildcardSegment};
    use std::marker::PhantomData;
    use tachys::renderer::dom::Dom;

    #[test]
    pub fn matches_single_root_route() {
        let routes = Routes::<_, Dom>::new(NestedRoute {
            segments: StaticSegment("/"),
            children: (),
            data: (),
            view: |_| (),
            rndr: PhantomData,
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
                view: |_| "Contact Me",
                rndr: PhantomData,
            },
            data: (),
            view: |_| "Home",
            rndr: PhantomData,
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
        assert_eq!(matched.to_child().unwrap().matched(), "/author/contact");

        let view = matched.to_view();
        assert_eq!(*view, "Home");
        assert_eq!(*matched.to_child().unwrap().to_view(), "Contact Me");
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
                rndr: PhantomData,
            },
            data: (),
            view: "Home",
            rndr: PhantomData,
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
                        rndr: PhantomData,
                    },
                    NestedRoute {
                        segments: StaticSegment("about"),
                        children: (),
                        data: (),
                        view: || (),
                        rndr: PhantomData,
                    },
                ),
                data: (),
                view: || (),
                rndr: PhantomData,
            },
            NestedRoute {
                segments: StaticSegment("/blog"),
                children: (
                    NestedRoute {
                        segments: StaticSegment(""),
                        children: (),
                        data: (),
                        view: || (),
                        rndr: PhantomData,
                    },
                    NestedRoute {
                        segments: (StaticSegment("post"), ParamSegment("id")),
                        children: (),
                        data: (),
                        view: || (),
                        rndr: PhantomData,
                    },
                ),
                data: (),
                view: || (),
                rndr: PhantomData,
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
                            rndr: PhantomData,
                        },
                        NestedRoute {
                            segments: StaticSegment("about"),
                            children: (),
                            data: (),
                            view: || (),
                            rndr: PhantomData,
                        },
                    ),
                    data: (),
                    view: || (),
                    rndr: PhantomData,
                },
                NestedRoute {
                    segments: StaticSegment("/blog"),
                    children: (
                        NestedRoute {
                            segments: StaticSegment(""),
                            children: (),
                            data: (),
                            view: || (),
                            rndr: PhantomData,
                        },
                        NestedRoute {
                            segments: StaticSegment("category"),
                            children: (),
                            data: (),
                            view: || (),
                            rndr: PhantomData,
                        },
                        NestedRoute {
                            segments: (
                                StaticSegment("post"),
                                ParamSegment("id"),
                            ),
                            children: (),
                            data: (),
                            view: || (),
                            rndr: PhantomData,
                        },
                    ),
                    data: (),
                    view: || (),
                    rndr: PhantomData,
                },
                NestedRoute {
                    segments: (
                        StaticSegment("/contact"),
                        WildcardSegment("any"),
                    ),
                    children: (),
                    data: (),
                    view: || (),
                    rndr: PhantomData,
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
