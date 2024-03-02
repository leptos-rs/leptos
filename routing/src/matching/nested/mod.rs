use super::{
    IntoParams, MatchNestedRoutes, PartialPathMatch, PossibleRouteMatch,
};
use crate::PathSegment;
use core::iter;

mod tuples;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct NestedRoute<Segments, Children, Data, ViewFn> {
    pub segments: Segments,
    pub children: Children,
    pub data: Data,
    pub view: ViewFn,
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

    pub fn child(&self) -> &Child {
        &self.child
    }
}

impl<'a, Segments, Children, Data, ViewFn> MatchNestedRoutes<'a>
    for NestedRoute<Segments, Children, Data, ViewFn>
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
