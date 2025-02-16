use crate::{
    matching::nested::any_nested_match::{AnyNestedMatch, IntoAnyNestedMatch},
    GeneratedRouteData, MatchNestedRoutes, RouteMatchId,
};
use std::{any::Any, fmt::Debug};
use tachys::prelude::IntoErased;

/// A type-erased container for any [`MatchNestedRoutes`].
pub struct AnyNestedRoute {
    value: Box<dyn Any + Send>,
    clone: fn(&Box<dyn Any + Send>) -> AnyNestedRoute,
    match_nested:
        for<'a> fn(
            &'a Box<dyn Any + Send>,
            &'a str,
        )
            -> (Option<(RouteMatchId, AnyNestedMatch)>, &'a str),
    generate_routes: fn(&Box<dyn Any + Send>) -> Vec<GeneratedRouteData>,
}

impl Clone for AnyNestedRoute {
    fn clone(&self) -> Self {
        (self.clone)(&self.value)
    }
}

impl Debug for AnyNestedRoute {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AnyNestedRoute").finish_non_exhaustive()
    }
}

impl IntoErased for AnyNestedRoute {
    type Output = Self;

    fn into_erased(self) -> Self::Output {
        self
    }
}

/// Converts anything implementing [`MatchNestedRoutes`] into [`AnyNestedRoute`].
pub trait IntoAnyNestedRoute {
    /// Wraps the nested route.
    fn into_any_nested_route(self) -> AnyNestedRoute;
}

impl<T> IntoAnyNestedRoute for T
where
    T: MatchNestedRoutes + Send + Clone + 'static,
{
    fn into_any_nested_route(self) -> AnyNestedRoute {
        AnyNestedRoute {
            value: Box::new(self),
            clone: |value| {
                value
                    .downcast_ref::<T>()
                    .unwrap()
                    .clone()
                    .into_any_nested_route()
            },
            match_nested: |value, path| {
                let (maybe_match, path) =
                    value.downcast_ref::<T>().unwrap().match_nested(path);
                (
                    maybe_match.map(|(id, matched)| {
                        (id, matched.into_any_nested_match())
                    }),
                    path,
                )
            },
            generate_routes: |value| {
                value
                    .downcast_ref::<T>()
                    .unwrap()
                    .generate_routes()
                    .into_iter()
                    .collect()
            },
        }
    }
}

impl MatchNestedRoutes for AnyNestedRoute {
    type Data = AnyNestedMatch;
    type Match = AnyNestedMatch;

    fn match_nested<'a>(
        &'a self,
        path: &'a str,
    ) -> (Option<(RouteMatchId, Self::Match)>, &'a str) {
        (self.match_nested)(&self.value, path)
    }

    fn generate_routes(&self) -> impl IntoIterator<Item = GeneratedRouteData> {
        (self.generate_routes)(&self.value)
    }
}
