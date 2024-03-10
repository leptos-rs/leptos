use crate::RouteData;
use either_of::*;
use tachys::{renderer::Renderer, view::Render};

pub trait ChooseView<R>
where
    Self: 'static,
    R: Renderer + 'static,
{
    type Output: Render<R>;

    fn choose(self, route_data: RouteData<R>) -> Self::Output;
}

impl<F, View, R> ChooseView<R> for F
where
    F: Fn(RouteData<R>) -> View + 'static,
    View: Render<R>,
    R: Renderer + 'static,
{
    type Output = View;

    fn choose(self, route_data: RouteData<R>) -> Self::Output {
        self(route_data)
    }
}

impl<R> ChooseView<R> for ()
where
    R: Renderer + 'static,
{
    type Output = ();

    fn choose(self, _route_data: RouteData<R>) -> Self::Output {}
}

impl<A, B, Rndr> ChooseView<Rndr> for Either<A, B>
where
    A: ChooseView<Rndr>,
    B: ChooseView<Rndr>,
    Rndr: Renderer + 'static,
{
    type Output = Either<A::Output, B::Output>;

    fn choose(self, route_data: RouteData<Rndr>) -> Self::Output {
        match self {
            Either::Left(f) => Either::Left(f.choose(route_data)),
            Either::Right(f) => Either::Right(f.choose(route_data)),
        }
    }
}

macro_rules! tuples {
    ($either:ident => $($ty:ident),*) => {
        impl<$($ty,)* Rndr> ChooseView<Rndr> for $either<$($ty,)*>
        where
            $($ty: ChooseView<Rndr>,)*
            Rndr: Renderer + 'static,
        {
            type Output = $either<$($ty::Output,)*>;

            fn choose(self, route_data: RouteData<Rndr>) -> Self::Output {
                match self {
                    $($either::$ty(f) => $either::$ty(f.choose(route_data)),)*
                }
            }
        }
    };
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
