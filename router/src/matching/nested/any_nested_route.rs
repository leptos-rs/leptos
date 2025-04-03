#![allow(clippy::type_complexity)]
use crate::{
    matching::nested::any_nested_match::{AnyNestedMatch, IntoAnyNestedMatch},
    GeneratedRouteData, MatchNestedRoutes, RouteMatchId,
};
use std::fmt::Debug;
use tachys::{erased::Erased, prelude::IntoMaybeErased};

/// A type-erased container for any [`MatchNestedRoutes`].
pub struct AnyNestedRoute {
    value: Erased,
    clone: fn(&Erased) -> AnyNestedRoute,
    match_nested:
        for<'a> fn(
            &'a Erased,
            &'a str,
        )
            -> (Option<(RouteMatchId, AnyNestedMatch)>, &'a str),
    generate_routes: fn(&Erased) -> Vec<GeneratedRouteData>,
    optional: fn(&Erased) -> bool,
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

impl IntoMaybeErased for AnyNestedRoute {
    type Output = Self;

    fn into_maybe_erased(self) -> Self::Output {
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
        fn clone<T: MatchNestedRoutes + Send + Clone + 'static>(
            value: &Erased,
        ) -> AnyNestedRoute {
            value.get_ref::<T>().clone().into_any_nested_route()
        }

        fn match_nested<'a, T: MatchNestedRoutes + Send + Clone + 'static>(
            value: &'a Erased,
            path: &'a str,
        ) -> (Option<(RouteMatchId, AnyNestedMatch)>, &'a str) {
            let (maybe_match, path) = value.get_ref::<T>().match_nested(path);
            (
                maybe_match
                    .map(|(id, matched)| (id, matched.into_any_nested_match())),
                path,
            )
        }

        fn generate_routes<T: MatchNestedRoutes + Send + Clone + 'static>(
            value: &Erased,
        ) -> Vec<GeneratedRouteData> {
            value.get_ref::<T>().generate_routes().into_iter().collect()
        }

        fn optional<T: MatchNestedRoutes + Send + Clone + 'static>(
            value: &Erased,
        ) -> bool {
            value.get_ref::<T>().optional()
        }

        AnyNestedRoute {
            value: Erased::new(self),
            clone: clone::<T>,
            match_nested: match_nested::<T>,
            generate_routes: generate_routes::<T>,
            optional: optional::<T>,
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

    fn optional(&self) -> bool {
        (self.optional)(&self.value)
    }
}
