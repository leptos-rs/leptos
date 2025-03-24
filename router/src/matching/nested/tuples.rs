use super::{MatchInterface, MatchNestedRoutes, PathSegment, RouteMatchId};
use crate::{ChooseView, GeneratedRouteData, MatchParams};
use core::iter;
use either_of::*;
use std::borrow::Cow;
use tachys::view::iterators::StaticVec;

impl MatchParams for () {
    fn to_params(&self) -> Vec<(Cow<'static, str>, String)> {
        Vec::new()
    }
}

impl MatchInterface for () {
    type Child = ();

    fn as_id(&self) -> RouteMatchId {
        RouteMatchId(0)
    }

    fn as_matched(&self) -> &str {
        ""
    }

    fn into_view_and_child(self) -> (impl ChooseView, Option<Self::Child>) {
        ((), None)
    }
}

impl MatchNestedRoutes for () {
    type Data = ();
    type Match = ();

    fn optional(&self) -> bool {
        false
    }

    fn match_nested<'a>(
        &self,
        path: &'a str,
    ) -> (Option<(RouteMatchId, Self::Match)>, &'a str) {
        (Some((RouteMatchId(0), ())), path)
    }

    fn generate_routes(
        &self,
    ) -> impl IntoIterator<Item = GeneratedRouteData> + '_ {
        iter::once(GeneratedRouteData {
            segments: vec![PathSegment::Unit],
            ..Default::default()
        })
    }
}

impl<A> MatchParams for (A,)
where
    A: MatchParams,
{
    fn to_params(&self) -> Vec<(Cow<'static, str>, String)> {
        self.0.to_params()
    }
}

impl<A> MatchInterface for (A,)
where
    A: MatchInterface + 'static,
{
    type Child = A::Child;

    fn as_id(&self) -> RouteMatchId {
        self.0.as_id()
    }

    fn as_matched(&self) -> &str {
        self.0.as_matched()
    }

    fn into_view_and_child(self) -> (impl ChooseView, Option<Self::Child>) {
        self.0.into_view_and_child()
    }
}

impl<A> MatchNestedRoutes for (A,)
where
    A: MatchNestedRoutes + 'static,
{
    type Data = A::Data;
    type Match = A::Match;

    fn match_nested<'a>(
        &'a self,
        path: &'a str,
    ) -> (Option<(RouteMatchId, Self::Match)>, &'a str) {
        self.0.match_nested(path)
    }

    fn generate_routes(
        &self,
    ) -> impl IntoIterator<Item = GeneratedRouteData> + '_ {
        self.0.generate_routes()
    }

    fn optional(&self) -> bool {
        self.0.optional()
    }
}

impl<A, B> MatchParams for Either<A, B>
where
    A: MatchParams,
    B: MatchParams,
{
    fn to_params(&self) -> Vec<(Cow<'static, str>, String)> {
        match self {
            Either::Left(i) => i.to_params(),
            Either::Right(i) => i.to_params(),
        }
    }
}

impl<A, B> MatchInterface for Either<A, B>
where
    A: MatchInterface,
    B: MatchInterface,
{
    type Child = Either<A::Child, B::Child>;

    fn as_id(&self) -> RouteMatchId {
        match self {
            Either::Left(i) => i.as_id(),
            Either::Right(i) => i.as_id(),
        }
    }

    fn as_matched(&self) -> &str {
        match self {
            Either::Left(i) => i.as_matched(),
            Either::Right(i) => i.as_matched(),
        }
    }

    fn into_view_and_child(self) -> (impl ChooseView, Option<Self::Child>) {
        match self {
            Either::Left(i) => {
                let (view, child) = i.into_view_and_child();
                (Either::Left(view), child.map(Either::Left))
            }
            Either::Right(i) => {
                let (view, child) = i.into_view_and_child();
                (Either::Right(view), child.map(Either::Right))
            }
        }
    }
}

impl<A, B> MatchNestedRoutes for (A, B)
where
    A: MatchNestedRoutes,
    B: MatchNestedRoutes,
{
    type Data = (A::Data, B::Data);
    type Match = Either<A::Match, B::Match>;

    fn match_nested<'a>(
        &'a self,
        path: &'a str,
    ) -> (Option<(RouteMatchId, Self::Match)>, &'a str) {
        #[allow(non_snake_case)]
        let (A, B) = &self;
        if let (Some((id, matched)), remaining) = A.match_nested(path) {
            return (Some((id, Either::Left(matched))), remaining);
        }
        if let (Some((id, matched)), remaining) = B.match_nested(path) {
            return (Some((id, Either::Right(matched))), remaining);
        }
        (None, path)
    }

    fn generate_routes(
        &self,
    ) -> impl IntoIterator<Item = GeneratedRouteData> + '_ {
        #![allow(non_snake_case)]

        let (A, B) = &self;

        let A = A.generate_routes().into_iter();
        let B = B.generate_routes().into_iter();

        A.chain(B)
    }

    fn optional(&self) -> bool {
        self.0.optional() && self.1.optional()
    }
}

impl<T> MatchNestedRoutes for StaticVec<T>
where
    T: MatchNestedRoutes,
{
    type Data = Vec<T::Data>;
    type Match = T::Match;

    fn match_nested<'a>(
        &'a self,
        path: &'a str,
    ) -> (Option<(RouteMatchId, Self::Match)>, &'a str) {
        for item in self.iter() {
            if let (Some((id, matched)), remaining) = item.match_nested(path) {
                return (Some((id, matched)), remaining);
            }
        }
        (None, path)
    }

    fn generate_routes(
        &self,
    ) -> impl IntoIterator<Item = GeneratedRouteData> + '_ {
        self.iter().flat_map(T::generate_routes)
    }

    fn optional(&self) -> bool {
        self.iter().all(|n| n.optional())
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

macro_rules! tuples {
    ($either:ident => $($ty:ident = $count:expr),*) => {
        impl<'a, $($ty,)*> MatchParams for $either <$($ty,)*>
        where
			$($ty: MatchParams),*,
        {
            fn to_params(&self) -> Vec<(Cow<'static, str>, String)> {
                match self {
                    $($either::$ty(i) => i.to_params(),)*
                }
            }
        }

        impl<$($ty,)*> MatchInterface for $either <$($ty,)*>
        where
            $($ty: MatchInterface + 'static),*,
        {
            type Child = $either<$($ty::Child,)*>;

            fn as_id(&self) -> RouteMatchId {
                match self {
                    $($either::$ty(i) => i.as_id(),)*
                }
            }

            fn as_matched(&self) -> &str {
                match self {
                    $($either::$ty(i) => i.as_matched(),)*
                }
            }

            fn into_view_and_child(
                self,
            ) -> (
                impl ChooseView,
                Option<Self::Child>,
            ) {
                match self {
                    $($either::$ty(i) => {
                        let (view, child) = i.into_view_and_child();
                        ($either::$ty(view), child.map($either::$ty))
                    })*
                }
            }
        }

        impl<$($ty),*> MatchNestedRoutes for ($($ty,)*)
        where
			$($ty: MatchNestedRoutes + 'static),*,
        {
            type Data = ($($ty::Data,)*);
            type Match = $either<$($ty::Match,)*>;

            fn optional(&self) -> bool {
                #[allow(non_snake_case)]
                let ($($ty,)*) = &self;
                $($ty.optional() &&)*
                true
            }

            fn match_nested<'a>(&'a self, path: &'a str) -> (Option<(RouteMatchId, Self::Match)>, &'a str) {
                #[allow(non_snake_case)]

                let ($($ty,)*) = &self;
                $(if let (Some((_, matched)), remaining) = $ty.match_nested(path) {
                    return (Some((RouteMatchId($count), $either::$ty(matched))), remaining);
                })*
                (None, path)
            }

            fn generate_routes(
                &self,
            ) -> impl IntoIterator<Item = GeneratedRouteData> + '_ {
                #![allow(non_snake_case)]

                let ($($ty,)*) = &self;
                $(let $ty = $ty.generate_routes().into_iter();)*
                chain_generated!($($ty,)*)
            }
        }
    }
}

tuples!(EitherOf3 => A = 0, B = 1, C = 2);
tuples!(EitherOf4 => A = 0, B = 1, C = 2, D = 3);
tuples!(EitherOf5 => A = 0, B = 1, C = 2, D = 3, E = 4);
tuples!(EitherOf6 => A = 0, B = 1, C = 2, D = 3, E = 4, F = 5);
tuples!(EitherOf7 => A = 0, B = 1, C = 2, D = 3, E = 4, F = 5, G = 6);
tuples!(EitherOf8 => A = 0, B = 1, C = 2, D = 3, E = 4, F = 5, G = 6, H = 7);
tuples!(EitherOf9 => A = 0, B = 1, C = 2, D = 3, E = 4, F = 5, G = 6, H = 7, I = 8);
tuples!(EitherOf10 => A = 0, B = 1, C = 2, D = 3, E = 4, F = 5, G = 6, H = 7, I = 8, J = 9);
tuples!(EitherOf11 => A = 0, B = 1, C = 2, D = 3, E = 4, F = 5, G = 6, H = 7, I = 8, J = 9, K = 10);
tuples!(EitherOf12 => A = 0, B = 1, C = 2, D = 3, E = 4, F = 5, G = 6, H = 7, I = 8, J = 9, K = 10, L = 11);
tuples!(EitherOf13 => A = 0, B = 1, C = 2, D = 3, E = 4, F = 5, G = 6, H = 7, I = 8, J = 9, K = 10, L = 11, M = 12);
tuples!(EitherOf14 => A = 0, B = 1, C = 2, D = 3, E = 4, F = 5, G = 6, H = 7, I = 8, J = 9, K = 10, L = 11, M = 12, N = 13);
tuples!(EitherOf15 => A = 0, B = 1, C = 2, D = 3, E = 4, F = 5, G = 6, H = 7, I = 8, J = 9, K = 10, L = 11, M = 12, N = 13, O = 14);
tuples!(EitherOf16 => A = 0, B = 1, C = 2, D = 3, E = 4, F = 5, G = 6, H = 7, I = 8, J = 9, K = 10, L = 11, M = 12, N = 13, O = 14, P = 15);
