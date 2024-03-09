use super::{
    MatchInterface, MatchNestedRoutes, PartialPathMatch, PathSegment,
    PossibleRouteMatch, RouteMatchId,
};
use crate::{ChooseView, MatchParams, RouteData};
use core::{fmt, iter};
use std::{borrow::Cow, marker::PhantomData};
use tachys::{
    renderer::Renderer,
    view::{Render, RenderHtml},
};

mod tuples;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct NestedRoute<Segments, Children, Data, ViewFn, R> {
    pub segments: Segments,
    pub children: Children,
    pub data: Data,
    pub view: ViewFn,
    pub rndr: PhantomData<R>,
}

impl<Segments, ViewFn, R> NestedRoute<Segments, (), (), ViewFn, R> {
    pub fn new<View>(path: Segments, view: ViewFn) -> Self
    where
        ViewFn: Fn(RouteData<R>) -> View,
        R: Renderer,
    {
        Self {
            segments: path,
            children: (),
            data: (),
            view,
            rndr: PhantomData,
        }
    }
}

impl<Segments, Data, ViewFn, R> NestedRoute<Segments, (), Data, ViewFn, R> {
    pub fn child<Children>(
        self,
        child: Children,
    ) -> NestedRoute<Segments, Children, Data, ViewFn, R> {
        let Self {
            segments,
            data,
            view,
            rndr,
            ..
        } = self;
        NestedRoute {
            segments,
            children: child,
            data,
            view,
            rndr,
        }
    }
}

#[derive(PartialEq, Eq)]
pub struct NestedMatch<ParamsIter, Child, ViewFn> {
    id: RouteMatchId,
    /// The portion of the full path matched only by this nested route.
    matched: String,
    /// The map of params matched only by this nested route.
    params: ParamsIter,
    /// The nested route.
    child: Child,
    view_fn: ViewFn,
}

impl<ParamsIter, Child, ViewFn> fmt::Debug
    for NestedMatch<ParamsIter, Child, ViewFn>
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

impl<ParamsIter, Child, ViewFn> MatchParams
    for NestedMatch<ParamsIter, Child, ViewFn>
where
    ParamsIter: IntoIterator<Item = (Cow<'static, str>, String)> + Clone,
{
    type Params = ParamsIter;

    #[inline(always)]
    fn to_params(&self) -> Self::Params {
        self.params.clone()
    }
}

impl<ParamsIter, Child, ViewFn, View, Rndr> MatchInterface<Rndr>
    for NestedMatch<ParamsIter, Child, ViewFn>
where
    Rndr: Renderer + 'static,
    Child: MatchInterface<Rndr> + MatchParams + 'static,
    ViewFn: Fn(RouteData<Rndr>) -> View + 'static,
    View: Render<Rndr> + RenderHtml<Rndr> + 'static,
{
    type Child = Child;
    type View = ViewFn::Output;

    fn as_id(&self) -> RouteMatchId {
        self.id
    }

    fn as_matched(&self) -> &str {
        &self.matched
    }

    fn into_view_and_child(
        self,
    ) -> (
        impl ChooseView<Rndr, Output = Self::View>,
        Option<Self::Child>,
    ) {
        (self.view_fn, Some(self.child))
    }
}

impl<Segments, Children, Data, ViewFn, View, Rndr> MatchNestedRoutes<Rndr>
    for NestedRoute<Segments, Children, Data, ViewFn, Rndr>
where
    Rndr: Renderer + 'static,
    Segments: PossibleRouteMatch,
    <<Segments as PossibleRouteMatch>::ParamsIter as IntoIterator>::IntoIter: Clone,
    Children: MatchNestedRoutes<Rndr>,
    <<<Children as MatchNestedRoutes<Rndr>>::Match as MatchParams>::Params as IntoIterator>::IntoIter: Clone,
   Children::Match: MatchParams,
   Children: 'static,
   <Children::Match as MatchParams>::Params: Clone,
    ViewFn: Fn(RouteData<Rndr>) -> View + Clone + 'static,
    View: Render<Rndr> + RenderHtml<Rndr> + 'static,
{
    type Data = Data;
    type View = View;
    type Match = NestedMatch<iter::Chain<
        <Segments::ParamsIter as IntoIterator>::IntoIter,
        <<Children::Match as MatchParams>::Params as IntoIterator>::IntoIter,
    >, Children::Match, ViewFn>;

    fn match_nested<'a>(
        &'a self,
        path: &'a str,
    ) -> (Option<(RouteMatchId, Self::Match)>, &'a str) {
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
                                    matched: matched.to_string(),
                                    params: params.chain(inner.to_params()),
                                    child: inner,
                                    view_fn: self.view.clone(),
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
