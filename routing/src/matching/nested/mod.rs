use super::{
    MatchInterface, MatchNestedRoutes, PartialPathMatch, PathSegment,
    PossibleRouteMatch, RouteMatchId,
};
use crate::{ChooseView, MatchParams};
use core::{fmt, iter};
use std::{borrow::Cow, marker::PhantomData, sync::atomic::{AtomicU16, Ordering}};
use either_of::Either;
use tachys::{
    renderer::Renderer,
    view::{Render, RenderHtml},
};

mod tuples;

static ROUTE_ID: AtomicU16 = AtomicU16::new(1);

#[derive(Debug, Copy, PartialEq, Eq)]
pub struct NestedRoute<Segments, Children, Data, ViewFn, R> {
    id: u16,
    pub segments: Segments,
    pub children: Option<Children>,
    pub data: Data,
    pub view: ViewFn,
    pub rndr: PhantomData<R>,
}

impl<Segments, Children, Data, ViewFn, R> Clone for NestedRoute<Segments, Children, Data, ViewFn, R> where Segments: Clone, Children: Clone, Data: Clone, ViewFn: Clone{
    fn clone(&self) -> Self {
        Self {
            id: self.id,segments: self.segments.clone(),children: self.children.clone(),data: self.data.clone(), view: self.view.clone(), rndr: PhantomData
        }
    }
}

impl<Segments, ViewFn, R> NestedRoute<Segments, (), (), ViewFn, R> {
    pub fn new<View>(path: Segments, view: ViewFn) -> Self
    where
        ViewFn: Fn() -> View,
        R: Renderer + 'static,
    {
        Self {
            id: ROUTE_ID.fetch_add(1, Ordering::Relaxed),
            segments: path,
            children: None,
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
            id,
            segments,
            data,
            view,
            rndr,
            ..
        } = self;
        NestedRoute {
            id,
            segments,
            children: Some(child),
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
    child: Option<Child>,
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
    ViewFn: Fn() -> View + Send + 'static,
    View: Render<Rndr> + RenderHtml<Rndr> + Send + 'static,
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
        (self.view_fn, self.child)
    }
}

impl<Segments, Children, Data, ViewFn, View, Rndr> MatchNestedRoutes<Rndr>
    for NestedRoute<Segments, Children, Data, ViewFn, Rndr>
where
    Self: 'static,
    Rndr: Renderer + 'static,
    Segments: PossibleRouteMatch + std::fmt::Debug,
    <<Segments as PossibleRouteMatch>::ParamsIter as IntoIterator>::IntoIter: Clone,
    Children: MatchNestedRoutes<Rndr>,
    <<<Children as MatchNestedRoutes<Rndr>>::Match as MatchParams>::Params as IntoIterator>::IntoIter: Clone,
   Children::Match: MatchParams,
   Children: 'static,
   <Children::Match as MatchParams>::Params: Clone,
    ViewFn: Fn() -> View + Send + Clone + 'static,
    View: Render<Rndr> + RenderHtml<Rndr> + Send + 'static,
{
    type Data = Data;
    type View = View;
    type Match = NestedMatch<iter::Chain<
        <Segments::ParamsIter as IntoIterator>::IntoIter,
        Either<iter::Empty::<
(Cow<'static, str>, String)
            >, <<Children::Match as MatchParams>::Params as IntoIterator>::IntoIter>
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
                    let (id, inner, remaining) = match &self.children {
                        None => (None, None, remaining),
                        Some(children) => {
                            let (inner, remaining) = children.match_nested(remaining);
                            let (id, inner) = inner?;
                           (Some(id), Some(inner), remaining) 
                        }
                    };
                    let params = params.into_iter();
                    let inner_params = match &inner {
                        None => Either::Left(iter::empty()),
                        Some(inner) => Either::Right(inner.to_params().into_iter())
                    };

                    let id = RouteMatchId(self.id);

                    if remaining.is_empty() || remaining == "/" {
                        Some((
                            Some((
                                id,
                                NestedMatch {
                                    id,
                                    matched: matched.to_string(),
                                    params: params.chain(inner_params),
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
        let children = self.children.as_ref();
        match children {
            None => Either::Left(iter::once(segment_routes)),
            Some(children) => {
                Either::Right(children.generate_routes().into_iter())
            }
        }
    }
}
