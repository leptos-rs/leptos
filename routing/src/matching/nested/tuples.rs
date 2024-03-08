use super::{MatchInterface, MatchNestedRoutes, PathSegment, RouteMatchId};
use crate::ChooseView;
use core::iter;
use either_of::*;
use tachys::renderer::Renderer;

impl<'a, Rndr> MatchInterface<'a, Rndr> for ()
where
    Rndr: Renderer,
{
    type Params = iter::Empty<(&'a str, &'a str)>;
    type Child = ();
    type View = ();

    fn as_id(&self) -> RouteMatchId {
        RouteMatchId(0)
    }

    fn as_matched(&self) -> &str {
        ""
    }

    fn to_params(&self) -> Self::Params {
        iter::empty()
    }

    fn into_view_and_child(
        self,
    ) -> (
        impl ChooseView<Rndr, Output = Self::View> + 'a,
        Option<Self::Child>,
    ) {
        ((), None)
    }
}

impl<Rndr> MatchNestedRoutes<Rndr> for ()
where
    Rndr: Renderer,
{
    type Data = ();
    type Match<'a> = ();

    fn match_nested<'a>(
        &self,
        path: &'a str,
    ) -> (Option<(RouteMatchId, Self::Match<'a>)>, &'a str) {
        (Some((RouteMatchId(0), ())), path)
    }

    fn generate_routes(
        &self,
    ) -> impl IntoIterator<Item = Vec<PathSegment>> + '_ {
        iter::once(vec![PathSegment::Unit])
    }
}

impl<'a, A, Rndr> MatchInterface<'a, Rndr> for (A,)
where
    A: MatchInterface<'a, Rndr>,
    Rndr: Renderer,
{
    type Params = A::Params;
    type Child = A::Child;
    type View = A::View;

    fn as_id(&self) -> RouteMatchId {
        RouteMatchId(0)
    }

    fn as_matched(&self) -> &str {
        self.0.as_matched()
    }

    fn to_params(&self) -> Self::Params {
        self.0.to_params()
    }

    fn into_view_and_child(
        self,
    ) -> (
        impl ChooseView<Rndr, Output = Self::View> + 'a,
        Option<Self::Child>,
    ) {
        self.0.into_view_and_child()
    }
}

impl<A, Rndr> MatchNestedRoutes<Rndr> for (A,)
where
    A: MatchNestedRoutes<Rndr>,
    Rndr: Renderer,
{
    type Data = A::Data;
    type Match<'a> = A::Match<'a> where A: 'a;

    fn match_nested<'a>(
        &'a self,
        path: &'a str,
    ) -> (Option<(RouteMatchId, Self::Match<'a>)>, &'a str) {
        self.0.match_nested(path)
    }

    fn generate_routes(
        &self,
    ) -> impl IntoIterator<Item = Vec<PathSegment>> + '_ {
        self.0.generate_routes()
    }
}

impl<'a, A, B, Rndr> MatchInterface<'a, Rndr> for Either<A, B>
where
    Rndr: Renderer,
    A: MatchInterface<'a, Rndr>,
    B: MatchInterface<'a, Rndr>,
{
    type Params = Either<
        <A::Params as IntoIterator>::IntoIter,
        <B::Params as IntoIterator>::IntoIter,
    >;
    type Child = Either<A::Child, B::Child>;
    type View = Either<A::View, B::View>;

    fn as_id(&self) -> RouteMatchId {
        match self {
            Either::Left(_) => RouteMatchId(0),
            Either::Right(_) => RouteMatchId(1),
        }
    }

    fn as_matched(&self) -> &str {
        match self {
            Either::Left(i) => i.as_matched(),
            Either::Right(i) => i.as_matched(),
        }
    }

    fn to_params(&self) -> Self::Params {
        match self {
            Either::Left(i) => Either::Left(i.to_params().into_iter()),
            Either::Right(i) => Either::Right(i.to_params().into_iter()),
        }
    }

    fn into_view_and_child(
        self,
    ) -> (
        impl ChooseView<Rndr, Output = Self::View> + 'a,
        Option<Self::Child>,
    ) {
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

impl<A, B, Rndr> MatchNestedRoutes<Rndr> for (A, B)
where
    A: MatchNestedRoutes<Rndr>,
    B: MatchNestedRoutes<Rndr>,
    Rndr: Renderer + 'static,
{
    type Data = (A::Data, B::Data);
    type Match<'a> = Either<A::Match<'a>, B::Match<'a>> where A: 'a, B: 'a;

    fn match_nested<'a>(
        &'a self,
        path: &'a str,
    ) -> (Option<(RouteMatchId, Self::Match<'a>)>, &'a str) {
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
    ) -> impl IntoIterator<Item = Vec<PathSegment>> + '_ {
        #![allow(non_snake_case)]

        let (A, B) = &self;

        let A = A.generate_routes().into_iter();
        let B = B.generate_routes().into_iter();

        A.chain(B)
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
        impl<'a, Rndr, $($ty,)*> MatchInterface<'a, Rndr> for $either <$($ty,)*>
        where
            Rndr: Renderer + 'static,
			$($ty: MatchInterface<'a, Rndr>),*,
			$($ty::Child: 'a),*,
        {
            type Params = $either<$(
                <$ty::Params as IntoIterator>::IntoIter,
            )*>;
            type Child = $either<$($ty::Child,)*>;
            type View = $either<$($ty::View,)*>;

            fn as_id(&self) -> RouteMatchId {
                match self {
                    $($either::$ty(_) => RouteMatchId($count),)*
                }
            }

            fn as_matched(&self) -> &str {
                match self {
                    $($either::$ty(i) => i.as_matched(),)*
                }
            }

            fn to_params(&self) -> Self::Params {
                match self {
                    $($either::$ty(i) => $either::$ty(i.to_params().into_iter()),)*
                }
            }

            fn into_child(self) -> Option<Self::Child> {
                Some(match self {
                    $($either::$ty(i) => $either::$ty(i.into_child()?),)*
                })
            }

            fn to_view(&self) -> impl ChooseView<Rndr, Output = Self::View> {
                match self {
                    $($either::$ty(i) => $either::$ty(i.to_view()),)*
                }
            }
        }

        impl<'a, Rndr, $($ty),*> MatchNestedRoutes<'a, Rndr> for ($($ty,)*)
        where
            Rndr: Renderer + 'static,
			$($ty: MatchNestedRoutes<'a, Rndr>),*,
			$($ty::Match: 'a),*,
        {
            type Data = ($($ty::Data,)*);
            type Match = $either<$($ty::Match,)*>;

            fn match_nested(&'a self, path: &'a str) -> (Option<(RouteMatchId, Self::Match)>, &'a str) {
                #[allow(non_snake_case)]

                let ($($ty,)*) = &self;
                let mut id = 0;
                $(if let (Some((_, matched)), remaining) = $ty.match_nested(path) {
                    return (Some((RouteMatchId(id), $either::$ty(matched))), remaining);
                } else {
                    id += 1;
                })*
                (None, path)
            }

            fn generate_routes(
                &self,
            ) -> impl IntoIterator<Item = Vec<PathSegment>> + '_ {
                #![allow(non_snake_case)]

                let ($($ty,)*) = &self;
                $(let $ty = $ty.generate_routes().into_iter();)*
                chain_generated!($($ty,)*)
            }
        }
    }
}
/*
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
*/
