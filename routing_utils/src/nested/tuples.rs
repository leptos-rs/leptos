use crate::{MatchInterface, MatchNestedRoutes, PathSegment};
use alloc::vec::Vec;
use core::iter;
use either_of::*;

impl<'a> MatchInterface<'a> for () {
    type Params = iter::Empty<(&'a str, &'a str)>;
    type Child = ();
    type View = ();

    fn to_params(&self) -> Self::Params {
        iter::empty()
    }

    fn to_child(&self) -> Self::Child {}

    fn to_view(&self) -> Self::View {}
}

impl<'a> MatchNestedRoutes<'a> for () {
    type Data = ();
    type Match = ();

    fn match_nested(&self, path: &'a str) -> (Option<Self::Match>, &'a str) {
        (Some(()), path)
    }

    fn generate_routes(
        &self,
    ) -> impl IntoIterator<Item = Vec<PathSegment>> + '_ {
        iter::once(vec![PathSegment::Unit])
    }
}

impl<'a, A> MatchInterface<'a> for (A,)
where
    A: MatchInterface<'a>,
{
    type Params = A::Params;
    type Child = A::Child;
    type View = A::View;

    fn to_params(&self) -> Self::Params {
        self.0.to_params()
    }

    fn to_child(&'a self) -> Self::Child {
        self.0.to_child()
    }

    fn to_view(&self) -> Self::View {
        self.0.to_view()
    }
}

impl<'a, A> MatchNestedRoutes<'a> for (A,)
where
    A: MatchNestedRoutes<'a>,
{
    type Data = A::Data;
    type Match = A::Match;

    fn match_nested(&'a self, path: &'a str) -> (Option<Self::Match>, &'a str) {
        self.0.match_nested(path)
    }

    fn generate_routes(
        &self,
    ) -> impl IntoIterator<Item = Vec<PathSegment>> + '_ {
        self.0.generate_routes()
    }
}

impl<'a, A, B> MatchInterface<'a> for Either<A, B>
where
    A: MatchInterface<'a>,
    B: MatchInterface<'a>,
{
    type Params = Either<
        <A::Params as IntoIterator>::IntoIter,
        <B::Params as IntoIterator>::IntoIter,
    >;
    type Child = Either<A::Child, B::Child>;
    type View = Either<A::View, B::View>;

    fn to_params(&self) -> Self::Params {
        match self {
            Either::Left(i) => Either::Left(i.to_params().into_iter()),
            Either::Right(i) => Either::Right(i.to_params().into_iter()),
        }
    }

    fn to_child(&'a self) -> Self::Child {
        match self {
            Either::Left(i) => Either::Left(i.to_child()),
            Either::Right(i) => Either::Right(i.to_child()),
        }
    }

    fn to_view(&self) -> Self::View {
        match self {
            Either::Left(i) => Either::Left(i.to_view()),
            Either::Right(i) => Either::Right(i.to_view()),
        }
    }
}

impl<'a, A, B> MatchNestedRoutes<'a> for (A, B)
where
    A: MatchNestedRoutes<'a>,
    B: MatchNestedRoutes<'a>,
{
    type Data = (A::Data, B::Data);
    type Match = Either<A::Match, B::Match>;

    fn match_nested(&'a self, path: &'a str) -> (Option<Self::Match>, &'a str) {
        #[allow(non_snake_case)]
        let (A, B) = &self;
        if let (Some(matched), remaining) = A.match_nested(path) {
            return (Some(Either::Left(matched)), remaining);
        }
        if let (Some(matched), remaining) = B.match_nested(path) {
            return (Some(Either::Right(matched)), remaining);
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
    ($either:ident => $($ty:ident),*) => {
        impl<'a, $($ty,)*> MatchInterface<'a> for $either <$($ty,)*>
        where
			$($ty: MatchInterface<'a>),*,
			$($ty::Child: 'a),*,
			$($ty::View: 'a),*,
        {
            type Params = $either<$(
                <$ty::Params as IntoIterator>::IntoIter,
            )*>;
            type Child = $either<$($ty::Child,)*>;
            type View = $either<$($ty::View,)*>;

            fn to_params(&self) -> Self::Params {
                match self {
                    $($either::$ty(i) => $either::$ty(i.to_params().into_iter()),)*
                }
            }

            fn to_child(&'a self) -> Self::Child {
                match self {
                    $($either::$ty(i) => $either::$ty(i.to_child()),)*
                }
            }

            fn to_view(&self) -> Self::View {
                match self {
                    $($either::$ty(i) => $either::$ty(i.to_view()),)*
                }
            }
        }

        impl<'a, $($ty),*> MatchNestedRoutes<'a> for ($($ty,)*)
        where
			$($ty: MatchNestedRoutes<'a>),*,
			$($ty::Match: 'a),*,
        {
            type Data = ($($ty::Data,)*);
            type Match = $either<$($ty::Match,)*>;

            fn match_nested(&'a self, path: &'a str) -> (Option<Self::Match>, &'a str) {
                #[allow(non_snake_case)]

                let ($($ty,)*) = &self;
                $(if let (Some(matched), remaining) = $ty.match_nested(path) {
                    return (Some($either::$ty(matched)), remaining);
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

tuples!(EitherOf3 => A, B, C);
tuples!(EitherOf4 => A, B, C, D);
tuples!(EitherOf5 => A, B, C, D, E);
tuples!(EitherOf6 => A, B, C, D, E, F);
tuples!(EitherOf7 => A, B, C, D, E, F, G);
tuples!(EitherOf8 => A, B, C, D, E, F, G, H);
tuples!(EitherOf9 => A, B, C, D, E, F, G, H, I);
tuples!(EitherOf10 => A, B, C, D, E, F, G, H, I, J);
tuples!(EitherOf11 => A, B, C, D, E, F, G, H, I, J, K);
tuples!(EitherOf12 => A, B, C, D, E, F, G, H, I, J, K, L);
tuples!(EitherOf13 => A, B, C, D, E, F, G, H, I, J, K, L, M);
tuples!(EitherOf14 => A, B, C, D, E, F, G, H, I, J, K, L, M, N);
tuples!(EitherOf15 => A, B, C, D, E, F, G, H, I, J, K, L, M, N, O);
tuples!(EitherOf16 => A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P);
