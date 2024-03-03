use super::{
    MatchInterface, MatchNestedRoutes, PartialPathMatch, PossibleRouteMatch,
};
use crate::PathSegment;
use core::iter;

mod tuples;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct NestedRoute<Segments, Children, Data, View> {
    pub segments: Segments,
    pub children: Children,
    pub data: Data,
    pub view: View,
}

#[derive(Debug, PartialEq, Eq)]
pub struct NestedMatch<'a, ParamsIter, Child, View> {
    /// The portion of the full path matched only by this nested route.
    matched: &'a str,
    /// The map of params matched only by this nested route.
    params: ParamsIter,
    /// The nested route.
    child: Child,
    view: &'a View,
}

impl<'a, ParamsIter, Child, View> MatchInterface<'a>
    for NestedMatch<'a, ParamsIter, Child, View>
where
    ParamsIter: IntoIterator<Item = (&'a str, &'a str)> + Clone,
    Child: 'a,
{
    type Params = ParamsIter;
    type Child = &'a Child;
    type View = &'a View;

    fn to_params(&self) -> Self::Params {
        self.params.clone()
    }

    fn to_child(&'a self) -> Self::Child {
        &self.child
    }

    fn to_view(&self) -> Self::View {
        self.view
    }
}

impl<'a, ParamsIter, Child, View> NestedMatch<'a, ParamsIter, Child, View> {
    pub fn matched(&self) -> &'a str {
        self.matched
    }
}

impl<'a, Segments, Children, Data, View> MatchNestedRoutes<'a>
    for NestedRoute<Segments, Children, Data, View>
where
    Segments: PossibleRouteMatch,
    Children: MatchNestedRoutes<'a>,
    <Segments::ParamsIter<'a> as IntoIterator>::IntoIter: Clone,
    <<Children::Match as MatchInterface<'a>>::Params as IntoIterator>::IntoIter:
        Clone,
    Children: 'a,
    View: 'a,
{
    type Data = Data;
    type Match = NestedMatch<'a, iter::Chain<
        <Segments::ParamsIter<'a> as IntoIterator>::IntoIter,
        <<Children::Match as MatchInterface<'a>>::Params as IntoIterator>::IntoIter,
    >, Children::Match, View>;

    fn match_nested(&'a self, path: &'a str) -> (Option<Self::Match>, &'a str) {
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
                                view: &self.view,
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
