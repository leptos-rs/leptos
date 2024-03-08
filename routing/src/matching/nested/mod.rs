use super::{
    MatchInterface, MatchNestedRoutes, PartialPathMatch, PathSegment,
    PossibleRouteMatch, RouteMatchId,
};
use crate::{ChooseView, RouteData};
use core::{fmt, iter};
use std::marker::PhantomData;
use tachys::{renderer::Renderer, view::Render};

mod tuples;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct NestedRoute<Segments, Children, Data, ViewFn, R> {
    pub segments: Segments,
    pub children: Children,
    pub data: Data,
    pub view: ViewFn,
    pub rndr: PhantomData<R>,
}

#[derive(PartialEq, Eq)]
pub struct NestedMatch<'a, ParamsIter, Child, ViewFn> {
    id: RouteMatchId,
    /// The portion of the full path matched only by this nested route.
    matched: &'a str,
    /// The map of params matched only by this nested route.
    params: ParamsIter,
    /// The nested route.
    child: Child,
    view_fn: &'a ViewFn,
}

impl<'a, ParamsIter, Child, ViewFn> fmt::Debug
    for NestedMatch<'a, ParamsIter, Child, ViewFn>
where
    ParamsIter: fmt::Debug,
    Child: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NestedMatch")
            .field("matched", &self.matched)
            .field("params", &self.params)
            .field("child", &self.child)
            .finish()
    }
}

impl<'a, ParamsIter, Child, ViewFn, Rndr> MatchInterface<'a, Rndr>
    for NestedMatch<'a, ParamsIter, Child, ViewFn>
where
    Rndr: Renderer + 'static,
    ParamsIter: IntoIterator<Item = (&'a str, &'a str)> + Clone,
    Child: MatchInterface<'a, Rndr>,
    ViewFn: Fn(RouteData<Rndr>),
    ViewFn::Output: Render<Rndr>,
{
    type Params = ParamsIter;
    type Child = Child;
    type View = ViewFn::Output;

    fn as_id(&self) -> RouteMatchId {
        self.id
    }

    fn as_matched(&self) -> &str {
        self.matched
    }

    fn to_params(&self) -> Self::Params {
        self.params.clone()
    }

    fn into_view_and_child(
        self,
    ) -> (
        impl ChooseView<Rndr, Output = Self::View> + 'a,
        Option<Self::Child>,
    ) {
        (self.view_fn, Some(self.child))
    }
}

impl<'a, ParamsIter, Child, ViewFn> NestedMatch<'a, ParamsIter, Child, ViewFn> {
    pub fn matched(&self) -> &'a str {
        self.matched
    }
}

impl<Segments, Children, Data, ViewFn, Rndr> MatchNestedRoutes<Rndr>
    for NestedRoute<Segments, Children, Data, ViewFn, Rndr>
where
    Rndr: Renderer + 'static,
    Segments: PossibleRouteMatch,
    Children: MatchNestedRoutes<Rndr>,
    for<'a> <Segments::ParamsIter<'a> as IntoIterator>::IntoIter: Clone,
    for <'a> <<Children::Match<'a> as MatchInterface<'a, Rndr>>::Params as IntoIterator>::IntoIter:
        Clone,
    ViewFn: Fn(RouteData<Rndr>),
{
    type Data = Data;
    type Match<'a> = NestedMatch<'a, iter::Chain<
        <Segments::ParamsIter<'a> as IntoIterator>::IntoIter,
        <<Children::Match<'a> as MatchInterface<'a, Rndr>>::Params as IntoIterator>::IntoIter,
    >, Children::Match<'a>, ViewFn> where <Children as MatchNestedRoutes<Rndr>>::Match<'a>: 'a, ViewFn: 'a, Children: 'a, Segments: 'a, Data: 'a;

    fn match_nested<'a>(
        &'a self,
        path: &'a str,
    ) -> (Option<(RouteMatchId, Self::Match<'a>)>, &'a str) {
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
                    let (id, inner) = inner?;
                    let params = params.into_iter();

                    if remaining.is_empty() || remaining == "/" {
                        Some((
                            Some((
                                id,
                                NestedMatch {
                                    id,
                                    matched,
                                    params: params.chain(inner.to_params()),
                                    child: inner,
                                    view_fn: &self.view,
                                },
                            )),
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
